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

/// Return a redacted raw row using command-specific secret-field knowledge.
#[must_use]
pub fn redact_command_row(command: &str, row: &Row) -> Row {
    let mut redacted = redact_row(row);
    if command == "/snmp/community/print" {
        if let Some(name) = redacted.get_mut("name") {
            "<redacted>".clone_into(name);
        }
    }
    redacted
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
        || contains_ascii(&key, b"community")
}

/// Return whether `needle` appears in `haystack`.
#[must_use]
pub fn contains_ascii(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|candidate| candidate == needle)
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::string::ToString as _;

    use super::*;

    #[test]
    fn command_redaction_hides_snmp_community_names() {
        let row = Row::from([
            ("name".to_string(), "community-secret".to_string()),
            ("addresses".to_string(), "0.0.0.0/0".to_string()),
        ]);

        let redacted = redact_command_row("/snmp/community/print", &row);

        assert_eq!(redacted.get("name").map(String::as_str), Some("<redacted>"));
        assert_eq!(redacted.get("addresses").map(String::as_str), Some("0.0.0.0/0"));
    }
}
