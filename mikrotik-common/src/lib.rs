#![no_std]

//! Shared helpers for `rust-mikrotik` crates.
//!
//! This crate contains small, stable building blocks that are needed by more
//! than one package, without tying those packages to higher-level client or
//! endpoint-model APIs.

extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeOwned;
use serde::de::Error as _;

/// A raw `RouterOS` API row keyed by `RouterOS` property name.
pub type Row = BTreeMap<String, String>;

/// Deserialize one raw `RouterOS` row into a typed endpoint response.
///
/// # Errors
///
/// Returns an error if the raw row cannot be converted to JSON or if the typed
/// endpoint model cannot be deserialized from that JSON value.
pub fn deserialize<T>(row: &Row) -> Result<T, serde_json::Error>
where
    T: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(row)?)
}

/// Deserialize a raw `RouterOS` row string field into an optional typed value.
///
/// # Errors
///
/// Returns an error if the field cannot be deserialized as an optional string or
/// if the present string cannot be parsed as `T`.
pub fn optional_from_str<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let value = Option::<String>::deserialize(deserializer)?;

    value.map(|value| value.parse().map_err(D::Error::custom)).transpose()
}

/// Deserialize a raw `RouterOS` boolean string field into an optional boolean.
///
/// # Errors
///
/// Returns an error if the field cannot be deserialized as an optional string.
pub fn optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;

    Ok(value.map(|value| matches!(value.as_str(), "true" | "yes")))
}

/// Deserialize a comma-separated `RouterOS` field into a list of strings.
///
/// # Errors
///
/// Returns an error if the field cannot be deserialized as an optional string.
pub fn comma_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;

    Ok(value.map_or_else(Vec::new, |value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }))
}

/// Deserialize a comma-separated `RouterOS` field into a typed list.
///
/// # Errors
///
/// Returns an error if the field cannot be deserialized as an optional string or
/// if any present list item cannot be parsed as `T`.
pub fn comma_list_from_str<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let value = Option::<String>::deserialize(deserializer)?;

    value.map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.parse().map_err(D::Error::custom))
                .collect()
        },
    )
}

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

/// Implement `Display` for a command enum with an inherent `as_path` method.
#[macro_export]
macro_rules! impl_command_display {
    ($type:ty) => {
        impl core::fmt::Display for $type {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(self.as_path())
            }
        }
    };
}

/// Emit one `trace` event prefixed with a label.
#[macro_export]
macro_rules! trace_with_label {
    ($label:expr, $($argument:tt)*) => {
        ::tracing::trace!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one `debug` event prefixed with a label.
#[macro_export]
macro_rules! debug_with_label {
    ($label:expr, $($argument:tt)*) => {
        ::tracing::debug!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one `info` event prefixed with a label.
#[macro_export]
macro_rules! info_with_label {
    ($label:expr, $($argument:tt)*) => {
        ::tracing::info!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one `warning` event prefixed with a label.
#[macro_export]
macro_rules! warn_with_label {
    ($label:expr, $($argument:tt)*) => {
        ::tracing::warn!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one `error` event prefixed with a label.
#[macro_export]
macro_rules! error_with_label {
    ($label:expr, $($argument:tt)*) => {
        ::tracing::error!("{}: {}", $label, format_args!($($argument)*));
    };
}
