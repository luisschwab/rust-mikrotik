//! Error and result types shared by the ISP tools modules.

use core::error;
use core::fmt;
use std::io;

/// Errors returned by ISP tools operations.
#[derive(Debug)]
pub enum Error {
    /// A lower-level `RouterOS` client operation failed.
    Client(mikrotik_client::error::Error),

    /// A filesystem or process I/O operation failed.
    Io(io::Error),

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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Client(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Graphviz { format, status } => {
                write!(f, "Graphviz failed while rendering {format}: {status}")
            }
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

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl Error {
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
            Self::Client(mikrotik_client::error::Error::Io(error)) => error.kind() == io::ErrorKind::TimedOut,
            Self::Io(error) => error.kind() == io::ErrorKind::TimedOut,
            Self::Client(_) | Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => false,
        }
    }

    /// Return true when the target actively refused the `RouterOS` API TCP connection.
    #[must_use]
    pub fn is_connection_refused(&self) -> bool {
        match self {
            Self::Client(mikrotik_client::error::Error::Io(error)) => error.kind() == io::ErrorKind::ConnectionRefused,
            Self::Client(_) | Self::Io(_) | Self::Graphviz { .. } | Self::InvalidTarget { .. } | Self::Target(_) => {
                false
            }
        }
    }
}

/// Result type used by this crate.
pub type Result<T> = core::result::Result<T, Error>;
