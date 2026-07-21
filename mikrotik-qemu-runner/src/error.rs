//! Error and result types for QEMU runner operations.

use core::error;
use core::fmt;
use std::io;
use std::path::PathBuf;

use mikrotik_client::error::Error as ClientError;

/// Result type used by `mikrotik-qemu-runner`.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by `mikrotik-qemu-runner`.
#[derive(Debug)]
pub enum Error {
    /// A filesystem or process I/O operation failed.
    Io {
        /// Operation that failed.
        operation: &'static str,
        /// Relevant path, when the operation involved one.
        path: Option<PathBuf>,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// A scenario or router configuration was invalid.
    Config(String),
    /// A required host tool was unavailable or failed.
    Tool(String),
    /// `RouterOS` client operation failed.
    Client(ClientError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => {
                if let Some(path) = path {
                    write!(f, "failed to {operation} {}: {source}", path.display())
                } else {
                    write!(f, "failed to {operation}: {source}")
                }
            }
            Self::Config(message) => write!(f, "QEMU runner configuration error: {message}"),
            Self::Tool(message) => write!(f, "QEMU runner host tool error: {message}"),
            Self::Client(error) => write!(f, "QEMU runner RouterOS client error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Client(error) => Some(error),
            Self::Config(_) | Self::Tool(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io {
            operation: "perform I/O operation",
            path: None,
            source,
        }
    }
}

impl From<ClientError> for Error {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

impl From<xshell::Error> for Error {
    fn from(error: xshell::Error) -> Self {
        Self::Tool(error.to_string())
    }
}

impl Error {
    /// Attach an operation and path to a filesystem error.
    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: Some(path.into()),
            source,
        }
    }
}
