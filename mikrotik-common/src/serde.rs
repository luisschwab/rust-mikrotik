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
use serde_json::Error as JsonError;
use serde_json::from_value;
use serde_json::to_value;

use crate::row::Row;

/// Deserialize one raw `RouterOS` row into a typed endpoint response.
///
/// # Errors
///
/// Returns an error if the raw row cannot be converted to JSON or if the typed
/// endpoint model cannot be deserialized from that JSON value.
pub fn deserialize<T>(row: &Row) -> Result<T, JsonError>
where
    T: DeserializeOwned,
{
    from_value(to_value(row)?)
}

/// Deserialize one raw `RouterOS` row while reporting attributes not consumed by `T`.
///
/// # Errors
///
/// Returns an error if the raw row cannot be converted to JSON or if the typed
/// endpoint model cannot be deserialized from that JSON value.
pub fn deserialize_with_ignored<T, F>(row: &Row, mut on_ignored: F) -> Result<T, JsonError>
where
    T: DeserializeOwned,
    F: FnMut(&str),
{
    let value = to_value(row)?;
    serde_ignored::deserialize(value, |path| {
        let path = path.to_string();
        on_ignored(&path);
    })
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
/// Returns an error if the field is neither a boolean nor one of `RouterOS`'s
/// boolean string spellings.
pub fn optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<StringOrBool>::deserialize(deserializer)?;

    value.map(StringOrBool::into_bool).transpose().map_err(D::Error::custom)
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
    fn into_bool(self) -> Result<bool, &'static str> {
        match self {
            Self::String(value) => match value.as_str() {
                "true" | "yes" => Ok(true),
                "false" | "no" => Ok(false),
                _ => Err("invalid RouterOS boolean"),
            },
            Self::Bool(value) => Ok(value),
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
    use alloc::string::String;
    use alloc::string::ToString as _;
    use alloc::vec;
    use alloc::vec::Vec;

    use serde::Deserialize;

    use crate::row::Row;

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

    /// Exercise every accepted wire and typed scalar representation.
    #[derive(Debug, Deserialize, PartialEq)]
    struct MixedScalars {
        /// Signed numeric scalar.
        #[serde(deserialize_with = "super::optional_from_str")]
        signed: Option<i64>,
        /// Floating-point scalar.
        #[serde(deserialize_with = "super::optional_from_str")]
        ratio: Option<f64>,
        /// Optional boolean scalar.
        #[serde(deserialize_with = "super::optional_bool")]
        enabled: Option<bool>,
        /// Untyped string list.
        #[serde(deserialize_with = "super::comma_list")]
        labels: Vec<String>,
        /// Typed scalar list.
        #[serde(deserialize_with = "super::comma_list_from_str")]
        counters: Vec<i64>,
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

    #[test]
    fn helpers_accept_null_strings_numbers_booleans_and_lists() {
        let typed = serde_json::from_str::<MixedScalars>(
            r#"{"signed":-7,"ratio":1.5,"enabled":false,"labels":["a","b"],"counters":[-1,2]}"#,
        )
        .unwrap();
        assert_eq!(
            typed,
            MixedScalars {
                signed: Some(-7),
                ratio: Some(1.5),
                enabled: Some(false),
                labels: vec!["a".to_string(), "b".to_string()],
                counters: vec![-1, 2],
            }
        );

        let wire = serde_json::from_str::<MixedScalars>(
            r#"{"signed":"-7","ratio":"1.5","enabled":"no","labels":" a, ,b ","counters":"-1, 2"}"#,
        )
        .unwrap();
        assert_eq!(wire, typed);

        let empty = serde_json::from_str::<MixedScalars>(
            r#"{"signed":null,"ratio":null,"enabled":null,"labels":null,"counters":null}"#,
        )
        .unwrap();
        assert_eq!(
            empty,
            MixedScalars {
                signed: None,
                ratio: None,
                enabled: None,
                labels: Vec::new(),
                counters: Vec::new(),
            }
        );
    }

    #[test]
    fn helpers_report_invalid_typed_values() {
        let invalid_scalar =
            serde_json::from_str::<Scalars>(r#"{"count":"not-a-number","enabled":true,"counters":[1]}"#);
        assert!(invalid_scalar.is_err());

        let invalid_list = serde_json::from_str::<Scalars>(r#"{"count":1,"enabled":true,"counters":["not-a-number"]}"#);
        assert!(invalid_list.is_err());

        let invalid_boolean = serde_json::from_str::<Scalars>(r#"{"enabled":"disabled"}"#);
        assert!(invalid_boolean.is_err());
    }

    #[test]
    fn raw_rows_deserialize_into_typed_models() {
        let row = Row::from([
            ("count".to_string(), "42".to_string()),
            ("enabled".to_string(), "true".to_string()),
            ("counters".to_string(), "1,2".to_string()),
        ]);

        assert_eq!(
            super::deserialize::<Scalars>(&row).unwrap(),
            Scalars {
                count: Some(42),
                enabled: Some(true),
                counters: vec![1, 2],
            }
        );
    }

    #[test]
    fn audited_raw_row_deserialization_reports_unconsumed_attributes() {
        let row = Row::from([
            ("count".to_string(), "42".to_string()),
            ("enabled".to_string(), "true".to_string()),
            ("counters".to_string(), "1,2".to_string()),
            ("future-field".to_string(), "value".to_string()),
        ]);
        let mut ignored = Vec::new();

        let scalars =
            super::deserialize_with_ignored::<Scalars, _>(&row, |path| ignored.push(path.to_string())).unwrap();

        assert_eq!(scalars.count, Some(42));
        assert_eq!(ignored, ["future-field"]);
    }
}
