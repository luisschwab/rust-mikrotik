//! Raw `RouterOS` row types.

use alloc::collections::BTreeMap;
use alloc::string::String;

/// A raw `RouterOS` API row keyed by `RouterOS` property name.
pub type Row = BTreeMap<String, String>;
