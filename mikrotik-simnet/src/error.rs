//! Error and result types for simnet operations.

use core::error;
use core::fmt;
use std::io;

/// Result type used by `mikrotik-simnet`.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by `mikrotik-simnet`.
#[derive(Debug)]
pub enum Error {
    /// Invalid command-line usage.
    Usage(String),
    /// Filesystem or process IO failed.
    Io(io::Error),
    /// A topology manifest was invalid.
    Manifest(String),
    /// A required host tool was unavailable or failed.
    Tool(String),
    /// `RouterOS` client operation failed.
    Client(mikrotik_client::Error),
    /// A check failed after routers booted.
    Check(String),
}

impl Error {
    /// Build a command-line usage error.
    #[must_use]
    pub fn usage(message: impl Into<String>) -> Self {
        Self::Usage(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(formatter, "{message}"),
            Self::Io(error) => write!(formatter, "simnet IO error: {error}"),
            Self::Manifest(message) => write!(formatter, "simnet topology error: {message}"),
            Self::Tool(message) => write!(formatter, "simnet host tool error: {message}"),
            Self::Client(error) => write!(formatter, "simnet RouterOS client error: {error}"),
            Self::Check(message) => write!(formatter, "simnet check failed: {message}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Client(error) => Some(error),
            Self::Usage(_) | Self::Manifest(_) | Self::Tool(_) | Self::Check(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<mikrotik_client::Error> for Error {
    fn from(error: mikrotik_client::Error) -> Self {
        Self::Client(error)
    }
}

impl From<xshell::Error> for Error {
    fn from(error: xshell::Error) -> Self {
        Self::Tool(error.to_string())
    }
}
