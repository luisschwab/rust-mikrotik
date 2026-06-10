//! SNMP endpoint rows.
//!
//! This module models `/snmp/*` configuration returned by `RouterOS`.

use alloc::string::String;
use core::net::IpAddr;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::ip::IpPrefix;

/// Response row from `/snmp/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Snmp {
    /// SNMP engine identifier.
    pub engine_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Source address or source address matcher.
    pub src_address: Option<IpAddr>,
    /// SNMP community used for traps.
    pub trap_community: Option<String>,
    /// SNMP trap generators that are enabled.
    pub trap_generators: Option<String>,
    /// SNMP protocol version used for traps.
    pub trap_version: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/snmp/community/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SnmpCommunity {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this snmp community.
    pub name: Option<String>,
    /// Authentication protocol used by the SNMP community.
    pub authentication_protocol: Option<String>,
    /// Encryption protocol used by the SNMP community.
    pub encryption_protocol: Option<String>,
    /// SNMP security level for the community.
    pub security: Option<String>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Address prefixes allowed to use the SNMP community.
    pub addresses: alloc::vec::Vec<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the SNMP community has read access.
    pub read_access: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the SNMP community has write access.
    pub write_access: Option<bool>,
}
