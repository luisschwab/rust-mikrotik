//! Serde helpers for raw `RouterOS` API rows.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::DeserializeOwned;
use serde::de::Error as _;

use crate::row::Row;

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
    let value = Option::<StringOrPrimitive>::deserialize(deserializer)?;

    value
        .map(|value| value.into_string().parse().map_err(D::Error::custom))
        .transpose()
}

/// String-like scalar accepted from both raw `RouterOS` rows and serialized typed snapshots.
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrPrimitive {
    /// `RouterOS` wire value.
    String(String),
    /// Serialized unsigned typed value.
    Unsigned(u64),
    /// Serialized signed typed value.
    Signed(i64),
    /// Serialized floating-point typed value.
    Float(f64),
}

impl StringOrPrimitive {
    /// Normalize the scalar through the existing `FromStr` conversion path.
    fn into_string(self) -> String {
        match self {
            Self::String(value) => value,
            Self::Unsigned(value) => value.to_string(),
            Self::Signed(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
        }
    }
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
    let value = Option::<StringOrBool>::deserialize(deserializer)?;

    Ok(value.map(StringOrBool::into_bool))
}

/// Boolean accepted from raw `RouterOS` strings and serialized typed snapshots.
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrBool {
    /// `RouterOS` wire value.
    String(String),
    /// Serialized typed value.
    Bool(bool),
}

impl StringOrBool {
    /// Convert accepted representations to a boolean.
    fn into_bool(self) -> bool {
        match self {
            Self::String(value) => matches!(value.as_str(), "true" | "yes"),
            Self::Bool(value) => value,
        }
    }
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
    let value = Option::<StringOrStringList>::deserialize(deserializer)?;

    Ok(value.map_or_else(Vec::new, StringOrStringList::into_list))
}

/// Comma-delimited wire string or typed serialized string list.
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrStringList {
    /// `RouterOS` wire value.
    String(String),
    /// Serialized typed value.
    List(Vec<String>),
}

impl StringOrStringList {
    /// Normalize both representations to a list.
    fn into_list(self) -> Vec<String> {
        match self {
            Self::String(value) => value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            Self::List(values) => values,
        }
    }
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
    let value = Option::<StringOrPrimitiveList>::deserialize(deserializer)?;

    value.map_or_else(
        || Ok(Vec::new()),
        |value| {
            value
                .into_strings()
                .into_iter()
                .map(|value| value.parse().map_err(D::Error::custom))
                .collect()
        },
    )
}

/// Comma-delimited wire string or typed serialized scalar list.
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrPrimitiveList {
    /// `RouterOS` wire value.
    String(String),
    /// Serialized typed values.
    List(Vec<StringOrPrimitive>),
}

impl StringOrPrimitiveList {
    /// Normalize both representations to parseable scalar strings.
    fn into_strings(self) -> Vec<String> {
        match self {
            Self::String(value) => value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            Self::List(values) => values.into_iter().map(StringOrPrimitive::into_string).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use serde::Deserialize;

    /// Exercise helpers through real derive-generated deserializers.
    #[derive(Debug, Deserialize, PartialEq)]
    struct Scalars {
        /// Numeric scalar accepted as a `RouterOS` string or JSON number.
        #[serde(deserialize_with = "super::optional_from_str")]
        count: Option<u64>,
        /// Boolean accepted as a `RouterOS` string or JSON boolean.
        #[serde(deserialize_with = "super::optional_bool")]
        enabled: Option<bool>,
        /// List accepted as a comma-delimited string or typed JSON array.
        #[serde(deserialize_with = "super::comma_list_from_str")]
        counters: Vec<u64>,
    }

    #[test]
    fn typed_snapshot_scalars_round_trip_through_wire_helpers() {
        let typed = serde_json::from_str::<Scalars>(r#"{"count":42,"enabled":true,"counters":[1,2]}"#).unwrap();
        assert_eq!(
            typed,
            Scalars {
                count: Some(42),
                enabled: Some(true),
                counters: vec![1, 2]
            }
        );

        let wire = serde_json::from_str::<Scalars>(r#"{"count":"42","enabled":"yes","counters":"1,2"}"#).unwrap();
        assert_eq!(wire, typed);
    }
}
