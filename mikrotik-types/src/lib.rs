#![no_std]

//! Versionless data models for `MikroTik`'s `RouterOS`.
//!
//! `RouterOS` API responses are maps of string properties. This crate provides
//! typed row structs and reusable primitive value types for deserializing those
//! maps while preserving the API's version and configuration variability with
//! optional fields.
//!
//! Response rows live under [`api`]. Reusable scalar and enum values live under
//! [`primitives`]. Higher-level observer/domain models, such as device
//! snapshots and topology, live outside both so they can compose API rows
//! without becoming wire-format types themselves.
//!
//! Endpoint row structs follow the `RouterOS` menu path in `PascalCase`,
//! dropping repeated namespace words only when the [`api`] submodule already
//! supplies that context. For example,
//! `/interface/ethernet/switch/port/print` maps to
//! [`api::interface::EthernetSwitchPort`], `/ip/settings/print` maps to
//! [`api::ip::IpSettings`], and `/routing/stats/origin/print` maps to
//! [`api::routing::RoutingStatsOrigin`].

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

pub mod api;
pub mod device;
pub mod primitives;
pub mod target;
pub mod topology;

pub use primitives::ParseError;
pub use primitives::RouterOsId;
pub(crate) use primitives::parse_non_empty;

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
