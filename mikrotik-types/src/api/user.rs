//! User endpoint rows.
//!
//! This module models local `RouterOS` user, group, and session rows.

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::system::RouterOsDateTime;
use crate::primitives::system::RouterOsDuration;

/// Response row from `/user/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct User {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this user.
    pub name: Option<String>,
    /// User group name.
    pub group: Option<String>,
    /// Comment configured on this user.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timestamp when the user last logged in.
    pub last_logged_in: Option<RouterOsDateTime>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this user account or counter entry has expired.
    pub expired: Option<bool>,
}

/// Response row from `/user/active/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ActiveUser {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this active user session.
    pub name: Option<String>,
    /// User group name.
    pub group: Option<String>,
    /// Remote address of the active user session.
    pub address: Option<String>,
    /// Management service used by the active session.
    pub via: Option<String>,
    /// Time when the active user session was established.
    pub when: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row came from or uses RADIUS.
    pub radius: Option<bool>,
}

/// Response row from `/user/group/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserGroup {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this user group.
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Policy names applied to this user group.
    pub policy: Vec<String>,
    /// User group skin name.
    pub skin: Option<String>,
}

/// Response row from `/user/aaa/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserAaa {
    /// Default user group assigned by this AAA configuration.
    pub default_group: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between interim accounting updates.
    pub interim_update: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether accounting is enabled.
    pub accounting: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether RADIUS integration is enabled for this row.
    pub use_radius: Option<bool>,
}

/// Response row from `/user/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UserSettings {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Minimum number of character categories required in passwords.
    pub minimum_categories: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Minimum password length required by user settings.
    pub minimum_password_length: Option<u32>,
}
