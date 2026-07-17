//! Error types returned by the client.

use core::fmt;
use std::error;
use std::io;

use mikrotik_common::redaction::redact_row;
use mikrotik_common::row::Row;

/// Result type used by the [`crate::client::Client`].
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by the `MikroTik` client.
#[derive(Debug)]
pub enum Error {
    /// Error returned by the network transport.
    Io(io::Error),
    /// Error returned by the sans-IO connection state machine.
    Connection(mikrotik_proto2::error::ConnectionError),
    /// Error returned by the `RouterOS` login handshake.
    Login(mikrotik_proto2::error::LoginError),
    /// The configured service protocol is not supported by this client.
    UnsupportedProtocol(&'static str),
    /// The transport closed before the current operation completed.
    ConnectionClosed,
    /// `RouterOS` returned a trap response while executing a command.
    Trap(String),
    /// `RouterOS` returned a fatal response while executing a command.
    Fatal(String),
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
            row: redact_row(row),
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
            Self::Io(error) => write!(f, "RouterOS transport error: {error}"),
            Self::Connection(error) => write!(f, "RouterOS protocol connection error: {error}"),
            Self::Login(error) => write!(f, "RouterOS login error: {error}"),
            Self::UnsupportedProtocol(protocol) => write!(f, "unsupported RouterOS protocol: {protocol}"),
            Self::ConnectionClosed => write!(f, "RouterOS connection closed"),
            Self::Trap(message) => write!(f, "RouterOS trap: {message}"),
            Self::Fatal(reason) => write!(f, "RouterOS fatal response: {reason}"),
            Self::Decode(error) => write!(f, "RouterOS row decode error: {error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Connection(error) => Some(error),
            Self::Login(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::ConnectionClosed | Self::Trap(_) | Self::Fatal(_) | Self::UnsupportedProtocol(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<mikrotik_proto2::error::ConnectionError> for Error {
    fn from(error: mikrotik_proto2::error::ConnectionError) -> Self {
        Self::Connection(error)
    }
}

impl From<mikrotik_proto2::error::LoginError> for Error {
    fn from(error: mikrotik_proto2::error::LoginError) -> Self {
        Self::Login(error)
    }
}

#[cfg(test)]
mod tests {
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
    }
}
