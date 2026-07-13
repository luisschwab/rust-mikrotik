//! Sensitive-value redaction helpers.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::row::Row;

/// Return a copy of a raw row with sensitive values replaced.
#[must_use]
pub fn redact_row(row: &Row) -> Row {
    row.iter()
        .map(|(key, value)| {
            let value = if is_sensitive_key(key) {
                "<redacted>".to_owned()
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect::<BTreeMap<_, _>>()
}

/// Return whether a `RouterOS` row key likely carries sensitive material.
#[must_use]
pub fn is_sensitive_key(key: &str) -> bool {
    let key = key
        .bytes()
        .filter_map(|byte| {
            if byte.is_ascii_alphanumeric() {
                Some(byte.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    contains_ascii(&key, b"password")
        || contains_ascii(&key, b"secret")
        || contains_ascii(&key, b"privatekey")
        || contains_ascii(&key, b"presharedkey")
        || contains_ascii(&key, b"authkey")
}

/// Return whether `needle` appears in `haystack`.
#[must_use]
pub fn contains_ascii(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|candidate| candidate == needle)
}
