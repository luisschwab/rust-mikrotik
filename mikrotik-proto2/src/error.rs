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
    },
    /// The required `message` attribute is missing from a trap response.
    MissingMessageAttribute,
}

impl fmt::Display for TrapCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "Invalid trap category value: {error}"),
            Self::OutOfRange(value) => write!(f, "Trap category out of range: {value} (valid range: 0-7)"),
            Self::InvalidAttribute { key } => write!(f, "Invalid trap attribute: key={key}"),
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

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;
    use alloc::string::String;
    use alloc::vec;

    use uuid::Uuid;

    use super::*;
    use crate::response::TrapCategory;
    use crate::tag::Tag;
    use crate::word::WordAttribute;
    use crate::word::WordCategory;

    const TEST_TAG: Tag = Tag::from_uuid(Uuid::from_bytes([
        0xa1, 0xa2, 0xa3, 0xa4, 0xb1, 0xb2, 0xc1, 0xc2, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
    ]));

    #[test]
    fn leaf_errors_have_actionable_messages() {
        assert_eq!(
            format!("{}", DecodeError::InvalidLengthPrefix(0xff)),
            "invalid length prefix byte: 0xff"
        );
        assert_eq!(format!("{}", SentenceError::PrefixLength), "Invalid prefix length");
        assert_eq!(format!("{}", MissingWord::Tag), "missing tag");
        assert_eq!(format!("{}", MissingWord::Category), "missing category");
        assert_eq!(format!("{}", MissingWord::Message), "missing message");
        assert_eq!(
            format!("{}", TrapCategoryError::OutOfRange(8)),
            "Trap category out of range: 8 (valid range: 0-7)"
        );
        assert_eq!(
            format!(
                "{}",
                TrapCategoryError::InvalidAttribute {
                    key: String::from("detail")
                }
            ),
            "Invalid trap attribute: key=detail"
        );
        assert_eq!(
            format!("{}", TrapCategoryError::MissingMessageAttribute),
            "Missing message attribute in trap response"
        );
    }

    #[test]
    fn word_types_are_derived_and_formatted() {
        let attribute = WordAttribute::try_from(b"=name=ether1".as_ref()).unwrap();
        let words = [
            (Word::Tag(TEST_TAG), WordType::Tag, "tag"),
            (Word::Category(WordCategory::Done), WordType::Category, "category"),
            (Word::Attribute(attribute), WordType::Attribute, "attribute"),
            (Word::Message("fatal"), WordType::Message, "message"),
        ];

        for (word, expected, display) in words {
            let word_type = WordType::from(word);
            assert_eq!(word_type, expected);
            assert_eq!(format!("{word_type}"), display);
        }
    }

    #[test]
    fn error_conversions_preserve_their_layer() {
        let word_error = Word::try_from(b"\xff".as_ref()).unwrap_err();
        let sentence = SentenceError::from(word_error);
        assert!(format!("{sentence}").starts_with("Word error: UTF-8 decoding error:"));

        let protocol = ProtocolError::from(sentence);
        assert!(format!("{protocol}").starts_with("Sentence error: Word error:"));
        assert!(matches!(ConnectionError::from(protocol), ConnectionError::Protocol(_)));
        assert!(matches!(
            ConnectionError::from(DecodeError::InvalidLengthPrefix(0xff)),
            ConnectionError::Decode(_)
        ));
        assert_eq!(format!("{}", ConnectionError::Closed), "connection is closed");

        let sequence = ProtocolError::WordSequence {
            word: WordType::Message,
            expected: vec![WordType::Tag, WordType::Attribute],
        };
        assert!(format!("{sequence}").contains("found Message, expected one of [Tag, Attribute]"));
        assert!(matches!(
            ProtocolError::from(MissingWord::Tag),
            ProtocolError::Incomplete(MissingWord::Tag)
        ));
        assert!(matches!(
            ProtocolError::from(TrapCategoryError::OutOfRange(9)),
            ProtocolError::TrapCategory(TrapCategoryError::OutOfRange(9))
        ));
    }

    #[test]
    fn login_errors_identify_authentication_fatal_and_wrapped_failures() {
        let trap = TrapResponse {
            tag: TEST_TAG,
            category: Some(TrapCategory::APIFailure),
            message: String::from("denied"),
        };
        assert!(format!("{}", LoginError::Authentication(trap)).contains("authentication failed: TrapResponse"));
        assert_eq!(
            format!("{}", LoginError::Fatal(String::from("shutdown"))),
            "fatal error during login: shutdown"
        );

        let protocol = ProtocolError::from(MissingWord::Category);
        assert!(format!("{}", LoginError::from(protocol)).starts_with("protocol error during login:"));
        assert!(format!("{}", LoginError::from(ConnectionError::Closed)).starts_with("connection error during login:"));
    }

    #[test]
    fn invalid_numeric_trap_category_retains_parse_context() {
        let parse_error = "invalid".parse::<u8>().unwrap_err();
        assert!(format!("{}", TrapCategoryError::Invalid(parse_error)).starts_with("Invalid trap category value:"));
    }
}
