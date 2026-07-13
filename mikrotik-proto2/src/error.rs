//! Error types for the MikroTik protocol implementation.
//!
//! This module provides a unified error hierarchy covering all levels of
//! protocol processing: wire-format decoding, word parsing, sentence parsing,
//! response parsing, connection state, and login.

use alloc::string::String;
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;
use core::num::ParseIntError;

use crate::response::TrapResponse;
use crate::word::Word;

/// Errors from the wire-format codec (length prefix decoding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// An invalid length prefix byte was encountered.
    InvalidLengthPrefix(u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLengthPrefix(byte) => write!(f, "invalid length prefix byte: 0x{byte:02x}"),
        }
    }
}

impl Error for DecodeError {}

/// Errors that can occur while processing a byte sequence into words within a sentence.
#[derive(Debug, PartialEq, Clone)]
pub enum SentenceError {
    /// A sequence of bytes could not be parsed into a valid [`Word`].
    WordError(crate::word::WordError),
    /// The prefix length of a sentence is incorrect or corrupt.
    PrefixLength,
}

impl fmt::Display for SentenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WordError(error) => write!(f, "Word error: {error}"),
            Self::PrefixLength => write!(f, "Invalid prefix length"),
        }
    }
}

impl From<crate::word::WordError> for SentenceError {
    fn from(error: crate::word::WordError) -> Self {
        Self::WordError(error)
    }
}

impl Error for SentenceError {}

/// Types of words that can be missing from a response.
#[derive(Debug, Clone, Copy)]
pub enum MissingWord {
    /// Missing `.tag`; all tagged responses must have a tag.
    Tag,
    /// Missing category (`!done`, `!re`, `!trap`, `!fatal`, `!empty`).
    Category,
    /// Missing message in a fatal response.
    Message,
}

impl fmt::Display for MissingWord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag => write!(f, "missing tag"),
            Self::Category => write!(f, "missing category"),
            Self::Message => write!(f, "missing message"),
        }
    }
}

impl Error for MissingWord {}

/// Discriminant for word types, used in error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordType {
    /// Tag word (`.tag=...`).
    Tag,
    /// Category word (`!done`, `!re`, etc.).
    Category,
    /// Attribute word (`=key=value`).
    Attribute,
    /// Message word (free-form text).
    Message,
}

impl fmt::Display for WordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tag => write!(f, "tag"),
            Self::Category => write!(f, "category"),
            Self::Attribute => write!(f, "attribute"),
            Self::Message => write!(f, "message"),
        }
    }
}

impl From<Word<'_>> for WordType {
    fn from(word: Word) -> Self {
        match word {
            Word::Tag(_) => Self::Tag,
            Word::Category(_) => Self::Category,
            Word::Attribute(_) => Self::Attribute,
            Word::Message(_) => Self::Message,
        }
    }
}

/// Errors that can occur while parsing trap categories in response sentences.
#[derive(Debug, Clone)]
pub enum TrapCategoryError {
    /// An invalid numeric value was encountered while parsing a trap category.
    Invalid(ParseIntError),
    /// The trap category number is out of the valid range (0-7).
    OutOfRange(u8),
    /// An unexpected attribute was found in a trap response.
    InvalidAttribute {
        /// The key of the invalid attribute.
        key: String,
        /// The value of the invalid attribute, if present.
        value: Option<String>,
    },
    /// The required `message` attribute is missing from a trap response.
    MissingMessageAttribute,
}

impl fmt::Display for TrapCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "Invalid trap category value: {error}"),
            Self::OutOfRange(value) => write!(f, "Trap category out of range: {value} (valid range: 0-7)"),
            Self::InvalidAttribute { key, value } => {
                write!(f, "Invalid trap attribute: key={key}, value={value:?}")
            }
            Self::MissingMessageAttribute => write!(f, "Missing message attribute in trap response"),
        }
    }
}

impl Error for TrapCategoryError {}

/// Errors that can occur while parsing a [`CommandResponse`](crate::response::CommandResponse)
/// from a decoded sentence.
#[derive(Debug, Clone)]
pub enum ProtocolError {
    /// Error within the sentence structure (word parsing or length prefix).
    Sentence(SentenceError),
    /// The response is missing required words to be valid.
    Incomplete(MissingWord),
    /// An unexpected word type was encountered in the response sequence.
    WordSequence {
        /// The unexpected [`WordType`] that was encountered.
        word: WordType,
        /// The expected [`WordType`] variants.
        expected: Vec<WordType>,
    },
    /// Error parsing or identifying a trap response category.
    TrapCategory(TrapCategoryError),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sentence(error) => write!(f, "Sentence error: {error}"),
            Self::Incomplete(error) => write!(f, "Incomplete response: {error}"),
            Self::WordSequence { word, expected } => {
                write!(f, "Unexpected word type: found {word:?}, expected one of {expected:?}")
            }
            Self::TrapCategory(error) => write!(f, "Trap category error: {error}"),
        }
    }
}

impl Error for ProtocolError {}

impl From<SentenceError> for ProtocolError {
    fn from(error: SentenceError) -> Self {
        Self::Sentence(error)
    }
}

impl From<MissingWord> for ProtocolError {
    fn from(error: MissingWord) -> Self {
        Self::Incomplete(error)
    }
}

impl From<TrapCategoryError> for ProtocolError {
    fn from(error: TrapCategoryError) -> Self {
        Self::TrapCategory(error)
    }
}

/// Errors from the [`Connection`](crate::connection::Connection) state machine.
#[derive(Debug, Clone)]
pub enum ConnectionError {
    /// A wire-format decoding error occurred.
    Decode(DecodeError),
    /// A protocol-level parsing error occurred.
    Protocol(ProtocolError),
    /// The connection has been fatally shut down and cannot accept new operations.
    Closed,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "decode error: {error}"),
            Self::Protocol(error) => write!(f, "protocol error: {error}"),
            Self::Closed => write!(f, "connection is closed"),
        }
    }
}

impl Error for ConnectionError {}

impl From<DecodeError> for ConnectionError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<ProtocolError> for ConnectionError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// Errors from the login handshake process.
#[derive(Debug, Clone)]
pub enum LoginError {
    /// The router rejected the login credentials.
    Authentication(TrapResponse),
    /// A fatal error occurred during login.
    Fatal(String),
    /// A protocol error occurred during login.
    Protocol(ProtocolError),
    /// A connection error occurred during login.
    Connection(ConnectionError),
}

impl fmt::Display for LoginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication(response) => write!(f, "authentication failed: {response}"),
            Self::Fatal(error) => write!(f, "fatal error during login: {error}"),
            Self::Protocol(error) => write!(f, "protocol error during login: {error}"),
            Self::Connection(error) => write!(f, "connection error during login: {error}"),
        }
    }
}

impl Error for LoginError {}

impl From<ProtocolError> for LoginError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<ConnectionError> for LoginError {
    fn from(error: ConnectionError) -> Self {
        Self::Connection(error)
    }
}
