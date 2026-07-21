//! Crawler error and retry classification types.

use core::error;
use core::fmt;
use core::time::Duration;
use std::io;
use std::io::ErrorKind;

use mikrotik_client::error::Error as ClientError;
use mikrotik_types::target::ObserverError;

/// Errors returned by ISP tools operations.
#[derive(Debug)]
pub enum Error {
    /// A lower-level `RouterOS` client operation failed.
    Client(ClientError),

    /// A required `RouterOS` command exceeded its deadline.
    CommandTimeout {
        /// Exact `RouterOS` API command path.
        command: String,
        /// Configured command deadline.
        duration: Duration,
    },

    /// A required `RouterOS` endpoint returned no rows.
    RequiredEndpointEmpty {
        /// Exact `RouterOS` API command path.
        command: String,
    },

    /// A filesystem or process I/O operation failed.
    Io(io::Error),

    /// A target address from the input or a neighbor row could not be used.
    InvalidTarget {
        /// Address that failed validation.
        address: String,
        /// Human-readable validation error.
        message: String,
    },

    /// A target could not be constructed.
    Target(ObserverError),
}

/// Retry-relevant category for a crawler failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// `RouterOS` rejected the configured credentials.
    InvalidCredentials,
    /// The API TCP port refused the connection.
    ApiRefused,
    /// The target network cannot be reached.
    NetworkUnreachable,
    /// A connect or command operation timed out.
    Timeout,
    /// The peer reset the connection.
    ConnectionReset,
    /// Any other failure.
    Other,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return match self.failure_kind() {
                FailureKind::InvalidCredentials => f.write_str("Invalid Credentials"),
                FailureKind::ApiRefused => f.write_str("API Refused Connection"),
                FailureKind::NetworkUnreachable => f.write_str("Network Unreachable"),
                FailureKind::Timeout => match self.timeout_duration() {
                    Some(duration) => write!(f, "Timed Out After {} seconds", duration.as_secs()),
                    None => f.write_str("Timed Out"),
                },
                FailureKind::ConnectionReset => f.write_str("Connection Reset"),
                FailureKind::Other => match self {
                    Self::Client(error) => write!(f, "{error}"),
                    Self::CommandTimeout { command, duration } => {
                        write!(f, "RouterOS command {command} exceeded {duration:?}")
                    }
                    Self::RequiredEndpointEmpty { command } => {
                        write!(f, "required RouterOS command {command} returned no rows")
                    }
                    Self::Io(error) => write!(f, "{error}"),
                    Self::InvalidTarget { address, message } => {
                        write!(f, "invalid target address {address:?}: {message}")
                    }
                    Self::Target(error) => write!(f, "target error: {error}"),
                },
            };
        }

        match self {
            Self::Client(error) => write!(f, "{error}"),
            Self::CommandTimeout { command, duration } => {
                write!(f, "RouterOS command {command} exceeded {duration:?}")
            }
            Self::RequiredEndpointEmpty { command } => {
                write!(f, "required RouterOS command {command} returned no rows")
            }
            Self::Io(error) => write!(f, "{error}"),
            Self::InvalidTarget { address, message } => {
                write!(f, "invalid target address {address:?}: {message}")
            }
            Self::Target(error) => write!(f, "target error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Client(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::CommandTimeout { .. }
            | Self::RequiredEndpointEmpty { .. }
            | Self::InvalidTarget { .. }
            | Self::Target(_) => None,
        }
    }
}

impl From<ClientError> for Error {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

impl From<ObserverError> for Error {
    fn from(error: ObserverError) -> Self {
        Self::Target(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl Error {
    /// Return the retry-relevant failure category.
    #[must_use]
    pub fn failure_kind(&self) -> FailureKind {
        if self.is_authentication_failure() {
            FailureKind::InvalidCredentials
        } else if self.is_connection_refused() {
            FailureKind::ApiRefused
        } else if self.is_network_unreachable() {
            FailureKind::NetworkUnreachable
        } else if self.is_timeout_failure() {
            FailureKind::Timeout
        } else if self.is_connection_reset() {
            FailureKind::ConnectionReset
        } else {
            FailureKind::Other
        }
    }

    /// Return true when this error represents rejected `RouterOS` credentials.
    #[must_use]
    pub fn is_authentication_failure(&self) -> bool {
        match self {
            Self::Client(error) => error.is_authentication_failure(),
            Self::CommandTimeout { .. }
            | Self::RequiredEndpointEmpty { .. }
            | Self::Io(_)
            | Self::InvalidTarget { .. }
            | Self::Target(_) => false,
        }
    }

    /// Return true when this error is a timeout while connecting to a target.
    #[must_use]
    pub fn is_timeout_failure(&self) -> bool {
        match self {
            Self::CommandTimeout { .. } | Self::Client(ClientError::Timeout { .. }) => true,
            Self::Client(ClientError::Transport { source, .. }) => source.kind() == ErrorKind::TimedOut,
            Self::Io(error) => error.kind() == ErrorKind::TimedOut,
            Self::Client(_) | Self::RequiredEndpointEmpty { .. } | Self::InvalidTarget { .. } | Self::Target(_) => {
                false
            }
        }
    }

    /// Return true when the target actively refused the `RouterOS` API TCP connection.
    #[must_use]
    pub fn is_connection_refused(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == ErrorKind::ConnectionRefused)
    }

    /// Return true when the target network cannot be reached.
    #[must_use]
    pub fn is_network_unreachable(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == ErrorKind::NetworkUnreachable)
    }

    /// Return true when the peer reset the connection.
    #[must_use]
    pub fn is_connection_reset(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == ErrorKind::ConnectionReset)
    }

    /// Return the configured deadline for a structured timeout failure.
    fn timeout_duration(&self) -> Option<Duration> {
        match self {
            Self::CommandTimeout { duration, .. } => Some(*duration),
            Self::Client(error) => error.timeout_duration(),
            Self::RequiredEndpointEmpty { .. } | Self::Io(_) | Self::InvalidTarget { .. } | Self::Target(_) => None,
        }
    }

    /// Return the wrapped I/O error, if any.
    fn io_error(&self) -> Option<&io::Error> {
        match self {
            Self::Client(error) => error.transport_error(),
            Self::Io(error) => Some(error),
            Self::CommandTimeout { .. }
            | Self::RequiredEndpointEmpty { .. }
            | Self::InvalidTarget { .. }
            | Self::Target(_) => None,
        }
    }
}

/// Result type used by this crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_display_formats_structured_timeout_reason() {
        let error = Error::CommandTimeout {
            command: "/system/resource/print".to_owned(),
            duration: Duration::from_secs(5),
        };

        assert_eq!(format!("{error:#}"), "Timed Out After 5 seconds");
    }

    #[test]
    fn alternate_display_formats_compact_connection_reason() {
        let error = Error::Io(io::Error::from(ErrorKind::ConnectionRefused));

        assert_eq!(format!("{error:#}"), "API Refused Connection");
    }

    #[test]
    fn command_timeout_display_includes_the_exact_command() {
        let error = Error::CommandTimeout {
            command: "/ip/firewall/filter/print".to_owned(),
            duration: Duration::from_secs(15),
        };

        assert_eq!(
            format!("{error}"),
            "RouterOS command /ip/firewall/filter/print exceeded 15s"
        );
    }
}
