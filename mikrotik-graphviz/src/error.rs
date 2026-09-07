//! Graphviz rendering and artifact I/O errors.

use core::error;
use core::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// Errors returned while rendering or writing graph artifacts.
#[derive(Debug)]
pub enum Error {
    /// An artifact could not be read or written.
    Io {
        /// Operation that failed.
        operation: &'static str,
        /// Artifact involved in the operation.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },

    /// The Graphviz process could not be started.
    StartGraphviz {
        /// DOT input path.
        input: PathBuf,
        /// Underlying process error.
        source: io::Error,
    },

    /// Graphviz exited unsuccessfully while rendering an artifact.
    Graphviz {
        /// Requested output format.
        format: String,
        /// DOT input path.
        input: PathBuf,
        /// Requested output path.
        output: PathBuf,
        /// Process exit status.
        status: ExitStatus,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "failed to {operation} {}: {source}", path.display()),
            Self::StartGraphviz { input, source } => {
                write!(f, "failed to start Graphviz for {}: {source}", input.display())
            }
            Self::Graphviz {
                format,
                input,
                output,
                status,
            } => write!(
                f,
                "Graphviz failed to render {} as {format} to {}: {status}",
                input.display(),
                output.display()
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::StartGraphviz { source, .. } => Some(source),
            Self::Graphviz { .. } => None,
        }
    }
}

/// Result type used by this crate.
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn io_display_includes_operation_and_path() {
        let error = Error::Io {
            operation: "read SVG",
            path: PathBuf::from("topology.svg"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };

        let message = error.to_string();
        assert!(message.contains("read SVG"));
        assert!(message.contains("topology.svg"));
        assert!(error.source().is_some());
    }

    #[test]
    fn start_and_exit_errors_report_artifact_context() {
        let start = Error::StartGraphviz {
            input: PathBuf::from("topology.dot"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        assert!(start.to_string().contains("failed to start Graphviz for topology.dot"));
        assert!(start.source().is_some());

        let status = std::process::Command::new("sh")
            .args(["-c", "exit 7"])
            .status()
            .unwrap();
        let graphviz = Error::Graphviz {
            format: "svg".to_owned(),
            input: PathBuf::from("topology.dot"),
            output: PathBuf::from("topology.svg"),
            status,
        };
        let message = graphviz.to_string();
        assert!(message.contains("topology.dot"));
        assert!(message.contains("as svg"));
        assert!(message.contains("topology.svg"));
        assert!(graphviz.source().is_none());
    }
}
