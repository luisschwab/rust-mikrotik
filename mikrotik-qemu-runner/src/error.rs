//! Error and result types for QEMU runner operations.

use core::error;
use core::fmt;
use std::io;

/// Result type used by `mikrotik-qemu-runner`.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by `mikrotik-qemu-runner`.
#[derive(Debug)]
pub enum Error {
    /// Filesystem or process IO failed.
    Io(io::Error),
    /// A scenario or router configuration was invalid.
    Config(String),
    /// A required host tool was unavailable or failed.
    Tool(String),
    /// `RouterOS` client operation failed.
    Client(mikrotik_client::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "QEMU runner IO error: {error}"),
            Self::Config(message) => write!(f, "QEMU runner configuration error: {message}"),
            Self::Tool(message) => write!(f, "QEMU runner host tool error: {message}"),
            Self::Client(error) => write!(f, "QEMU runner RouterOS client error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Client(error) => Some(error),
            Self::Config(_) | Self::Tool(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<mikrotik_client::error::Error> for Error {
    fn from(error: mikrotik_client::error::Error) -> Self {
        Self::Client(error)
    }
}

impl From<xshell::Error> for Error {
    fn from(error: xshell::Error) -> Self {
        Self::Tool(error.to_string())
    }
}
