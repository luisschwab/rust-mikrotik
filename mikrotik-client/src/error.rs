//! Error types returned by the client.

use core::fmt;
use core::time::Duration;
use std::error;
use std::io;

use mikrotik_common::redaction::redact_command_row;
use mikrotik_common::row::Row;
use mikrotik_proto2::error::ConnectionError;
use mikrotik_proto2::error::LoginError;
use mikrotik_proto2::response::TrapCategory;

/// Result type used by the [`crate::client::Client`].
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by the `MikroTik` client.
#[derive(Debug)]
pub enum Error {
    /// Error returned by the network transport.
    Transport {
        /// Exact command being executed, if the transport failed after login.
        command: Option<String>,
        /// Underlying transport error.
        source: io::Error,
    },
    /// A bounded operation exceeded its configured duration.
    Timeout {
        /// Operation that timed out.
        operation: &'static str,
        /// Configured timeout duration.
        duration: Duration,
    },
    /// Error returned by the sans-IO connection state machine.
    Connection {
        /// Exact command being executed, if the protocol failed after login.
        command: Option<String>,
        /// Underlying protocol connection error.
        source: ConnectionError,
    },
    /// Error returned by the `RouterOS` login handshake.
    Login(LoginError),
    /// The configured service protocol is not supported by this client.
    UnsupportedProtocol(&'static str),
    /// The transport closed before the current operation completed.
    ConnectionClosed {
        /// Exact command being executed, if the connection closed after login.
        command: Option<String>,
    },
    /// `RouterOS` denied permission to execute a command.
    PermissionDenied {
        /// Exact command that was rejected.
        command: String,
        /// Trap message returned by the device.
        message: String,
    },
    /// `RouterOS` does not support a requested command.
    UnsupportedCommand {
        /// Exact command that is unsupported.
        command: String,
        /// Trap message returned by the device.
        message: String,
    },
    /// `RouterOS` returned a trap response while executing a command.
    Trap {
        /// Exact command that returned the trap.
        command: String,
        /// Trap category returned by the device, if present.
        category: Option<TrapCategory>,
        /// Trap message returned by the device.
        message: String,
    },
    /// `RouterOS` returned a fatal response while executing a command.
    Fatal {
        /// Exact command that was active when the fatal response arrived.
        command: String,
        /// Fatal reason returned by the device.
        reason: String,
    },
    /// A raw `RouterOS` row could not be decoded into its endpoint type.
    Decode(DecodeError),
}

/// Details about a raw row that failed typed deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// Command whose row failed typed decoding.
    command: String,
    /// Zero-based row index within the command reply.
    row_index: usize,
    /// Human-readable deserializer error.
    message: String,
    /// Redacted copy of the raw row.
    row: Row,
}

impl DecodeError {
    /// Build a decode error and redact sensitive values from the row context.
    pub(crate) fn new(command: &str, row_index: usize, message: String, row: &Row) -> Self {
        Self {
            command: command.to_owned(),
            row_index,
            message,
            row: redact_command_row(command, row),
        }
    }

    /// Return the command whose row failed to decode.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Return the zero-based index of the row that failed to decode.
    #[must_use]
    pub const fn row_index(&self) -> usize {
        self.row_index
    }

    /// Return the decode error message from the typed row parser.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Return the redacted raw row that failed to decode.
    #[must_use]
    pub fn row(&self) -> &Row {
        &self.row
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "command {} row {} failed to decode: {}; row: {:?}",
            self.command, self.row_index, self.message, self.row
        )
    }
}

impl error::Error for DecodeError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport {
                command: Some(command),
                source,
            } => write!(f, "RouterOS transport error while running {command}: {source}"),
            Self::Transport { command: None, source } => write!(f, "RouterOS transport error: {source}"),
            Self::Timeout { operation, duration } => write!(f, "{operation} exceeded {duration:?}"),
            Self::Connection {
                command: Some(command),
                source,
            } => write!(f, "RouterOS protocol error while running {command}: {source}"),
            Self::Connection { command: None, source } => write!(f, "RouterOS protocol connection error: {source}"),
            Self::Login(error) => write!(f, "RouterOS login error: {error}"),
            Self::UnsupportedProtocol(protocol) => write!(f, "unsupported RouterOS protocol: {protocol}"),
            Self::ConnectionClosed { command: Some(command) } => {
                write!(f, "RouterOS connection closed while running {command}")
            }
            Self::ConnectionClosed { command: None } => write!(f, "RouterOS connection closed"),
            Self::PermissionDenied { command, message } => {
                write!(f, "RouterOS denied permission to run {command}: {message}")
            }
            Self::UnsupportedCommand { command, message } => {
                write!(f, "RouterOS command {command} is unsupported: {message}")
            }
            Self::Trap {
                command,
                category,
                message,
            } => match category {
                Some(category) => write!(f, "RouterOS trap while running {command} ({category:?}): {message}"),
                None => write!(f, "RouterOS trap while running {command}: {message}"),
            },
            Self::Fatal { command, reason } => {
                write!(f, "RouterOS fatal response while running {command}: {reason}")
            }
            Self::Decode(error) => write!(f, "RouterOS row decode error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Transport { source, .. } => Some(source),
            Self::Connection { source, .. } => Some(source),
            Self::Login(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::Timeout { .. }
            | Self::ConnectionClosed { .. }
            | Self::PermissionDenied { .. }
            | Self::UnsupportedCommand { .. }
            | Self::Trap { .. }
            | Self::Fatal { .. }
            | Self::UnsupportedProtocol(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Transport {
            command: None,
            source: error,
        }
    }
}

impl From<ConnectionError> for Error {
    fn from(error: ConnectionError) -> Self {
        Self::Connection {
            command: None,
            source: error,
        }
    }
}

impl Error {
    /// Return the exact command path associated with this failure, if any.
    #[must_use]
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Transport { command, .. } | Self::Connection { command, .. } | Self::ConnectionClosed { command } => {
                command.as_deref()
            }
            Self::PermissionDenied { command, .. }
            | Self::UnsupportedCommand { command, .. }
            | Self::Trap { command, .. }
            | Self::Fatal { command, .. } => Some(command),
            Self::Timeout { .. } | Self::Login(_) | Self::UnsupportedProtocol(_) | Self::Decode(_) => None,
        }
    }

    /// Return the device-provided message for a command trap classification.
    #[must_use]
    pub fn trap_message(&self) -> Option<&str> {
        match self {
            Self::PermissionDenied { message, .. }
            | Self::UnsupportedCommand { message, .. }
            | Self::Trap { message, .. } => Some(message),
            Self::Transport { .. }
            | Self::Timeout { .. }
            | Self::Connection { .. }
            | Self::Login(_)
            | Self::UnsupportedProtocol(_)
            | Self::ConnectionClosed { .. }
            | Self::Fatal { .. }
            | Self::Decode(_) => None,
        }
    }

    /// Return the underlying transport error, if this is a transport failure.
    #[must_use]
    pub const fn transport_error(&self) -> Option<&io::Error> {
        match self {
            Self::Transport { source, .. } => Some(source),
            _ => None,
        }
    }

    /// Return the configured timeout duration, if this is a timeout failure.
    #[must_use]
    pub const fn timeout_duration(&self) -> Option<Duration> {
        match self {
            Self::Timeout { duration, .. } => Some(*duration),
            _ => None,
        }
    }

    /// Return whether `RouterOS` rejected the login credentials.
    #[must_use]
    pub const fn is_authentication_failure(&self) -> bool {
        matches!(self, Self::Login(LoginError::Authentication(_)))
    }

    /// Attach an exact command path to an error raised during command execution.
    pub(crate) fn with_command(self, command: &str) -> Self {
        match self {
            Self::Transport { source, .. } => Self::Transport {
                command: Some(command.to_owned()),
                source,
            },
            Self::Connection { source, .. } => Self::Connection {
                command: Some(command.to_owned()),
                source,
            },
            Self::ConnectionClosed { .. } => Self::ConnectionClosed {
                command: Some(command.to_owned()),
            },
            error => error,
        }
    }
}

impl From<LoginError> for Error {
    fn from(error: LoginError) -> Self {
        Self::Login(error)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::error::Error as _;
    use std::io::ErrorKind;

    use mikrotik_proto2::Tag;
    use mikrotik_proto2::error::MissingWord;
    use mikrotik_proto2::error::ProtocolError;
    use mikrotik_proto2::response::TrapResponse;

    use super::*;

    #[test]
    fn decode_error_display_includes_context_and_redacts_sensitive_fields() {
        let row = Row::from([
            ("name".to_owned(), "peer-a".to_owned()),
            ("private-key".to_owned(), "secret-private-key".to_owned()),
            ("preshared-key".to_owned(), "secret-preshared-key".to_owned()),
            ("public-key".to_owned(), "not-redacted".to_owned()),
        ]);

        let error = DecodeError::new("/interface/wireguard/peers/print", 2, "invalid value".to_owned(), &row);
        let display = error.to_string();

        assert!(display.contains("/interface/wireguard/peers/print"));
        assert!(display.contains("row 2"));
        assert!(display.contains("invalid value"));
        assert!(display.contains("peer-a"));
        assert!(display.contains("<redacted>"));
        assert!(display.contains("not-redacted"));
        assert!(!display.contains("secret-private-key"));
        assert!(!display.contains("secret-preshared-key"));

        assert_eq!(error.command(), "/interface/wireguard/peers/print");
        assert_eq!(error.row_index(), 2);
        assert_eq!(error.message(), "invalid value");
        assert_eq!(error.row().get("private-key").map(String::as_str), Some("<redacted>"));
    }

    #[test]
    fn command_errors_include_the_exact_command() {
        let error = Error::PermissionDenied {
            command: "/ip/address/print".to_owned(),
            message: "not enough permissions".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "RouterOS denied permission to run /ip/address/print: not enough permissions"
        );
    }

    #[test]
    fn every_public_error_variant_has_contextual_display_output() {
        let cases = [
            (
                Error::Transport {
                    command: None,
                    source: io::Error::new(ErrorKind::BrokenPipe, "closed"),
                },
                "RouterOS transport error: closed",
            ),
            (
                Error::Transport {
                    command: Some("/ip/route/print".to_owned()),
                    source: io::Error::new(ErrorKind::BrokenPipe, "closed"),
                },
                "RouterOS transport error while running /ip/route/print: closed",
            ),
            (
                Error::Timeout {
                    operation: "RouterOS command",
                    duration: Duration::from_secs(2),
                },
                "RouterOS command exceeded 2s",
            ),
            (
                Error::Connection {
                    command: None,
                    source: ConnectionError::Closed,
                },
                "RouterOS protocol connection error: connection is closed",
            ),
            (
                Error::Connection {
                    command: Some("/system/resource/print".to_owned()),
                    source: ConnectionError::Closed,
                },
                "RouterOS protocol error while running /system/resource/print: connection is closed",
            ),
            (
                Error::Login(LoginError::Fatal("failed".to_owned())),
                "RouterOS login error: fatal error during login: failed",
            ),
            (Error::UnsupportedProtocol("ssh"), "unsupported RouterOS protocol: ssh"),
            (Error::ConnectionClosed { command: None }, "RouterOS connection closed"),
            (
                Error::ConnectionClosed {
                    command: Some("/system/identity/print".to_owned()),
                },
                "RouterOS connection closed while running /system/identity/print",
            ),
            (
                Error::UnsupportedCommand {
                    command: "/future/print".to_owned(),
                    message: "no such command".to_owned(),
                },
                "RouterOS command /future/print is unsupported: no such command",
            ),
            (
                Error::Trap {
                    command: "/test/print".to_owned(),
                    category: None,
                    message: "failed".to_owned(),
                },
                "RouterOS trap while running /test/print: failed",
            ),
            (
                Error::Trap {
                    command: "/test/print".to_owned(),
                    category: Some(TrapCategory::GeneralFailure),
                    message: "failed".to_owned(),
                },
                "RouterOS trap while running /test/print (GeneralFailure): failed",
            ),
            (
                Error::Fatal {
                    command: "/test/print".to_owned(),
                    reason: "shutdown".to_owned(),
                },
                "RouterOS fatal response while running /test/print: shutdown",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }

        let decode = DecodeError::new("/test/print", 0, "invalid".to_owned(), &Row::new());
        assert!(
            Error::Decode(decode)
                .to_string()
                .starts_with("RouterOS row decode error:")
        );
    }

    #[test]
    fn accessors_and_sources_distinguish_error_categories() {
        let transport = Error::from(io::Error::new(ErrorKind::ConnectionReset, "reset"));
        assert_eq!(transport.command(), None);
        assert_eq!(
            transport.transport_error().map(io::Error::kind),
            Some(ErrorKind::ConnectionReset)
        );
        assert!(transport.source().is_some());

        let timeout = Error::Timeout {
            operation: "test",
            duration: Duration::from_secs(3),
        };
        assert_eq!(timeout.timeout_duration(), Some(Duration::from_secs(3)));
        assert!(timeout.transport_error().is_none());
        assert!(timeout.source().is_none());

        let connection = Error::from(ConnectionError::from(ProtocolError::from(MissingWord::Tag)));
        assert!(connection.source().is_some());
        assert_eq!(connection.command(), None);

        let login = Error::from(LoginError::Fatal("fatal".to_owned()));
        assert!(login.source().is_some());
        assert!(!login.is_authentication_failure());

        let auth = Error::from(LoginError::Authentication(TrapResponse {
            tag: Tag::new(),
            category: None,
            message: "denied".to_owned(),
        }));
        assert!(auth.is_authentication_failure());

        let decode = Error::Decode(DecodeError::new("/test", 0, "bad row".to_owned(), &Row::new()));
        assert!(decode.source().is_some());
        assert_eq!(decode.timeout_duration(), None);
    }

    #[test]
    fn command_and_trap_accessors_cover_command_scoped_errors() {
        for error in [
            Error::PermissionDenied {
                command: "/a".to_owned(),
                message: "denied".to_owned(),
            },
            Error::UnsupportedCommand {
                command: "/a".to_owned(),
                message: "missing".to_owned(),
            },
            Error::Trap {
                command: "/a".to_owned(),
                category: None,
                message: "trap".to_owned(),
            },
        ] {
            assert_eq!(error.command(), Some("/a"));
            assert!(error.trap_message().is_some());
        }
        let fatal = Error::Fatal {
            command: "/a".to_owned(),
            reason: "fatal".to_owned(),
        };
        assert_eq!(fatal.command(), Some("/a"));
        assert_eq!(fatal.trap_message(), None);
        assert_eq!(Error::UnsupportedProtocol("ssh").command(), None);
    }

    #[test]
    fn command_context_is_attached_only_to_transport_connection_and_closed_errors() {
        let transport = Error::from(io::Error::new(ErrorKind::BrokenPipe, "closed")).with_command("/test/print");
        assert_eq!(transport.command(), Some("/test/print"));

        let connection = Error::from(ConnectionError::Closed).with_command("/test/print");
        assert_eq!(connection.command(), Some("/test/print"));

        let closed = Error::ConnectionClosed { command: None }.with_command("/test/print");
        assert_eq!(closed.command(), Some("/test/print"));

        let timeout = Error::Timeout {
            operation: "test",
            duration: Duration::ZERO,
        }
        .with_command("/test/print");
        assert_eq!(timeout.command(), None);
    }
}
