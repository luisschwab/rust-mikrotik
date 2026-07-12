//! Error and result types shared by the ISP tools modules.

use core::error;
use core::fmt;

/// Errors returned by ISP tools operations.
#[derive(Debug)]
pub enum Error {
    /// A lower-level `RouterOS` client operation failed.
    Client(mikrotik_client::error::Error),

    /// A filesystem or process I/O operation failed.
    Io(std::io::Error),

    /// Graphviz exited unsuccessfully while rendering an artifact.
    Graphviz {
        /// Requested output format.
        format: String,
        /// Process exit status.
        status: std::process::ExitStatus,
    },

    /// A target address from the input or a neighbor row could not be used.
    InvalidTarget {
        /// Address that failed validation.
        address: String,
        /// Human-readable validation error.
        message: String,
    },

    /// A target could not be constructed.
    Target(mikrotik_types::target::ObserverError),
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
                FailureKind::Timeout => write!(f, "Timed Out After {} seconds", self.timeout_seconds().unwrap_or(0)),
                FailureKind::ConnectionReset => f.write_str("Connection Reset"),
                FailureKind::Other => match self {
                    Self::Client(error) => write!(f, "{error}"),
                    Self::Io(error) => write!(f, "{error}"),
                    Self::Graphviz { format, status } => {
                        write!(f, "Graphviz failed while rendering {format}: {status}")
                    }
                    Self::InvalidTarget { address, message } => {
                        write!(f, "invalid target address {address:?}: {message}")
                    }
                    Self::Target(error) => write!(f, "target error: {error}"),
                },
            };
        }

        match self {
            Self::Client(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Graphviz { format, status } => {
                write!(f, "Graphviz failed while rendering {format}: {status}")
            }
            Self::InvalidTarget { address, message } => {
                write!(f, "Invalid Target Address {address:?}: {message}")
            }
            Self::Target(error) => write!(f, "Target Error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Client(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => None,
        }
    }
}

impl From<mikrotik_client::error::Error> for Error {
    fn from(error: mikrotik_client::error::Error) -> Self {
        Self::Client(error)
    }
}

impl From<mikrotik_types::target::ObserverError> for Error {
    fn from(error: mikrotik_types::target::ObserverError) -> Self {
        Self::Target(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
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
            Self::Client(mikrotik_client::error::Error::Login(error)) => {
                error.to_string().starts_with("authentication failed:")
            }
            Self::Client(_) | Self::Io(_) | Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => {
                false
            }
        }
    }

    /// Return true when this error is a timeout while connecting to a target.
    #[must_use]
    pub fn is_timeout_failure(&self) -> bool {
        match self {
            Self::Client(mikrotik_client::error::Error::Io(error)) => error.kind() == std::io::ErrorKind::TimedOut,
            Self::Io(error) => error.kind() == std::io::ErrorKind::TimedOut,
            Self::Client(_) | Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => false,
        }
    }

    /// Return true when the target actively refused the `RouterOS` API TCP connection.
    #[must_use]
    pub fn is_connection_refused(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::ConnectionRefused)
    }

    /// Return true when the target network cannot be reached.
    #[must_use]
    pub fn is_network_unreachable(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::NetworkUnreachable)
    }

    /// Return true when the peer reset the connection.
    #[must_use]
    pub fn is_connection_reset(&self) -> bool {
        self.io_error()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::ConnectionReset)
    }

    /// Return the configured timeout duration in seconds when it is embedded in the error text.
    fn timeout_seconds(&self) -> Option<u64> {
        let message = self.io_error()?.to_string();
        message
            .split_whitespace()
            .find_map(|token| token.strip_suffix('s')?.parse::<f64>().ok())
            .map(|seconds| seconds.round() as u64)
    }

    /// Return the wrapped I/O error, if any.
    fn io_error(&self) -> Option<&std::io::Error> {
        match self {
            Self::Client(mikrotik_client::error::Error::Io(error)) | Self::Io(error) => Some(error),
            Self::Client(_) | Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => None,
        }
    }
}

/// Result type used by this crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_display_formats_compact_timeout_reason() {
        let error = Error::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "connect attempt exceeded 5s",
        ));

        assert_eq!(format!("{error:#}"), "Timed Out After 5 seconds");
    }

    #[test]
    fn alternate_display_formats_compact_connection_reason() {
        let error = Error::Io(std::io::Error::from(std::io::ErrorKind::ConnectionRefused));

        assert_eq!(format!("{error:#}"), "API Refused Connection");
    }
}
