//! Command tag, a unique identifier for correlating commands with responses.
//!
//! Each command sent to a `MikroTik` router is assigned a [`Tag`] that the
//! router echoes back in all response sentences (`.tag=<value>`). This allows
//! multiplexing multiple in-flight commands over a single connection.
//!
//! `Tag` is a newtype wrapper around [`uuid::Uuid`] that provides type safety:
//! you cannot accidentally pass a random `Uuid` where a command tag is expected.

use core::fmt;
use core::str::FromStr;

use uuid::Uuid;

/// A unique tag identifying a command for response correlation.
///
/// Tags can be created explicitly via [`Tag::new`].
/// With the `std` feature enabled, [`Tag::new`] generates UUID v4 tags.
///
/// The router echoes the tag in every response sentence belonging to the command,
/// allowing the [`Connection`](crate::connection::Connection) to demultiplex responses.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag(Uuid);

impl Tag {
    /// Generate a new random tag (UUID v4).
    #[cfg(feature = "std")]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a tag from a caller-provided [`uuid::Uuid`].
    #[cfg(not(feature = "std"))]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create a tag from an existing [`uuid::Uuid`] in a const context.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Encode the tag as a hyphenated lowercase string into the provided buffer.
    ///
    /// The buffer must be at least 36 bytes. Returns the written slice.
    pub fn encode_lower<'a>(&self, buf: &'a mut [u8]) -> &'a str {
        self.0.as_hyphenated().encode_lower(buf)
    }

    /// Parse a tag from ASCII bytes without requiring UTF-8 validation.
    ///
    /// This is the fast path used by word parsing. The `.tag=<uuid>` value
    /// on the wire is always ASCII hex digits and hyphens, so we can skip
    /// `from_utf8` entirely and hand the bytes directly to the UUID parser.
    ///
    /// # Errors
    ///
    /// Returns [`uuid::Error`] if the bytes are not a valid UUID.
    pub fn try_from_ascii_bytes(bytes: &[u8]) -> Result<Self, uuid::Error> {
        Uuid::try_parse_ascii(bytes).map(Self)
    }
}

#[cfg(feature = "std")]
impl Default for Tag {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({})", self.0.as_hyphenated())
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

impl From<Uuid> for Tag {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<Tag> for Uuid {
    fn from(tag: Tag) -> Self {
        tag.0
    }
}

/// Parse a tag from a hyphenated UUID string, such as `.tag=...` wire data.
impl FromStr for Tag {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Uuid>().map(Self)
    }
}
