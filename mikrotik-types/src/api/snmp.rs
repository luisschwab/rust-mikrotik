//! SNMP endpoint rows.
//!
//! This module models `/snmp/*` configuration returned by `RouterOS`.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::ip::IpPrefix;

/// Response row from `/snmp/print`.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Snmp {
    /// SNMP engine identifier.
    pub engine_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Source address or source address matcher.
    pub src_address: Option<IpAddr>,
    /// SNMP community used for traps.
    #[serde(skip_serializing)]
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

impl fmt::Debug for Snmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snmp")
            .field("engine_id", &self.engine_id)
            .field("src_address", &self.src_address)
            .field("trap_community", &self.trap_community.as_ref().map(|_| "<redacted>"))
            .field("trap_generators", &self.trap_generators)
            .field("trap_version", &self.trap_version)
            .field("vrf", &self.vrf)
            .field("enabled", &self.enabled)
            .finish()
    }
}

/// Response row from `/snmp/community/print`.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SnmpCommunity {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this snmp community.
    #[serde(skip_serializing)]
    pub name: Option<String>,
    /// Authentication protocol used by the SNMP community.
    pub authentication_protocol: Option<String>,
    /// Encryption protocol used by the SNMP community.
    pub encryption_protocol: Option<String>,
    /// SNMP security level for the community.
    pub security: Option<String>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Address prefixes allowed to use the SNMP community.
    pub addresses: Vec<IpPrefix>,
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

impl fmt::Debug for SnmpCommunity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnmpCommunity")
            .field("id", &self.id)
            .field("name", &self.name.as_ref().map(|_| "<redacted>"))
            .field("authentication_protocol", &self.authentication_protocol)
            .field("encryption_protocol", &self.encryption_protocol)
            .field("security", &self.security)
            .field("addresses", &self.addresses)
            .field("default", &self.default)
            .field("disabled", &self.disabled)
            .field("read_access", &self.read_access)
            .field("write_access", &self.write_access)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString as _;

    use super::Snmp;
    use super::SnmpCommunity;

    #[test]
    fn community_values_are_not_formatted_or_serialized() {
        let settings = Snmp {
            trap_community: Some("trap-secret".to_string()),
            ..Snmp::default()
        };
        let community = SnmpCommunity {
            name: Some("community-secret".to_string()),
            ..SnmpCommunity::default()
        };

        let debug = format!("{settings:?} {community:?}");
        let serialized = format!(
            "{} {}",
            serde_json::to_string(&settings).unwrap(),
            serde_json::to_string(&community).unwrap()
        );

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("trap-secret"));
        assert!(!debug.contains("community-secret"));
        assert!(!serialized.contains("trap-secret"));
        assert!(!serialized.contains("community-secret"));
        assert!(!serialized.contains("trap-community"));
        assert!(!serialized.contains("name"));
    }
}
