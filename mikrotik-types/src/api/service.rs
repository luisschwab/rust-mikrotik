//! Miscellaneous service and platform endpoint rows.
//!
//! This module holds small `RouterOS` menu families that do not naturally fit
//! the interface, IP, routing, system, tool, queue, SNMP, or user modules.

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::InterfaceName;
use crate::primitives::system::RouterOsByteSize;
use crate::primitives::system::RouterOsDateTime;
use crate::primitives::system::RouterOsVersion;

/// Response row from `/caps-man/aaa/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CapsManAaa {
    /// RADIUS Called-Station-Id formatting mode.
    pub called_format: Option<String>,
    /// RADIUS MAC address formatting mode.
    pub mac_format: Option<String>,
    /// RADIUS MAC address case and separator mode.
    pub mac_mode: Option<String>,
    /// Interval between interim accounting updates.
    pub interim_update: Option<String>,
    /// How long `CAPsMAN` caches MAC authentication results.
    pub mac_caching: Option<String>,
}

/// Response row from `/caps-man/manager/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CapsManManager {
    /// Certificate authority used by the `CAPsMAN` manager.
    pub ca_certificate: Option<String>,
    /// Certificate associated with the service.
    pub certificate: Option<String>,
    /// `CAPsMAN` package upgrade policy for managed devices.
    pub upgrade_policy: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `CAPsMAN` peers must present a certificate.
    pub require_peer_certificate: Option<bool>,
}

/// Response row from `/caps-man/manager/interface/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CapsManManagerInterface {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or `RoMON` port is forbidden.
    pub forbid: Option<bool>,
}

/// Response row from `/certificate/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Certificate {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Certificate name.
    pub name: Option<String>,
    /// Certificate common name.
    pub common_name: Option<String>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Certificate subject alternative names.
    pub subject_alt_name: Vec<String>,
    /// Certificate issuer.
    pub issuer: Option<String>,
    /// Certificate serial number.
    pub serial_number: Option<String>,
    /// Certificate fingerprint.
    pub fingerprint: Option<String>,
    /// Private key algorithm.
    pub key_type: Option<String>,
    /// Private key size in bits or elliptic curve name.
    pub key_size: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Certificate validity period in days.
    pub days_valid: Option<u32>,
    /// Earliest validity timestamp.
    pub invalid_before: Option<String>,
    /// Latest validity timestamp.
    pub invalid_after: Option<String>,
    /// Remaining time before this certificate expires.
    pub expires_after: Option<String>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Certificate key usage flags.
    pub key_usage: Vec<String>,
    /// CA CRL host.
    pub ca_crl_host: Option<String>,
    /// Trust store selection.
    pub trust_store: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate has CRL data.
    pub crl: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate uses a smart-card key.
    pub smart_card_key: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate is a certificate authority.
    pub authority: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate has been issued.
    pub issued: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate has been revoked.
    pub revoked: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate is expired.
    pub expired: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this certificate is trusted.
    pub trusted: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the private key is present.
    pub private_key: Option<bool>,
}

/// Response row from `/certificate/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct CertificateSettings {
    /// Certificate revocation list download policy.
    pub crl_download: Option<String>,
    /// Storage location for certificate revocation lists.
    pub crl_store: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether certificate revocation lists are checked.
    pub crl_use: Option<bool>,
}

/// Response row from `/console/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConsoleSettings {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether console output sanitizes item names.
    pub sanitize_names: Option<bool>,
}

/// Response row from `/disk/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DiskSettings {
    /// Interface used for automatic media sharing.
    pub auto_media_interface: Option<String>,
    /// SMB user used for automatic disk sharing.
    pub auto_smb_user: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether automatic media sharing is enabled for disks.
    pub auto_media_sharing: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether automatic SMB sharing is enabled for disks.
    pub auto_smb_sharing: Option<bool>,
}

/// Response row from `/disk/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Disk {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Disk or partition name.
    pub name: Option<String>,
    /// Disk slot or identifier.
    pub slot: Option<String>,
    /// Disk model.
    pub model: Option<String>,
    /// Filesystem type.
    pub file_system: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Disk size.
    pub size: Option<RouterOsByteSize>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Free disk space.
    pub free: Option<RouterOsByteSize>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this disk is disabled.
    pub disabled: Option<bool>,
}

/// Response row from `/file/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct File {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this file.
    pub name: Option<String>,
    #[serde(rename = "type")]
    /// Type of file stored on the router filesystem.
    pub file_type: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Creation timestamp reported by `RouterOS`.
    pub creation_time: Option<RouterOsDateTime>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Package size.
    pub size: Option<RouterOsByteSize>,
}

/// Response row from `/mpls/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MplsSettings {
    /// MPLS label range reserved for dynamically allocated labels.
    pub dynamic_label_range: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes forwarded through MPLS fast path.
    pub mpls_fast_path_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets forwarded through MPLS fast path.
    pub mpls_fast_path_packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether fast path forwarding is allowed.
    pub allow_fast_path: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether MPLS propagates packet TTL values.
    pub propagate_ttl: Option<bool>,
}

/// Response row from `/partitions/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Partition {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Partition selected as fallback boot target.
    pub fallback_to: Option<String>,
    /// Name of this partition.
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Package size.
    pub size: Option<RouterOsByteSize>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// `RouterOS` version associated with this entry.
    pub version: Option<RouterOsVersion>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is active.
    pub active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
}

/// Response row from `/ppp/aaa/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PppAaa {
    /// Interval between interim accounting updates.
    pub interim_update: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether accounting is enabled.
    pub accounting: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv6 traffic is included in PPP accounting.
    pub enable_ipv6_accounting: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether PPP uses circuit-id in the NAS-Port-Id attribute.
    pub use_circuit_id_in_nas_port_id: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether RADIUS integration is enabled for this row.
    pub use_radius: Option<bool>,
}

/// Response row from `/ppp/profile/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PppProfile {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this ppp profile.
    pub name: Option<String>,
    /// Bridge learning mode used by the PPP profile.
    pub bridge_learning: Option<String>,
    /// TCP MSS adjustment mode used by the PPP profile.
    pub change_tcp_mss: Option<String>,
    /// Whether only one PPP session is allowed for the same user.
    pub only_one: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Compression policy used by the PPP profile.
    pub use_compression: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Encryption policy used by the PPP profile.
    pub use_encryption: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// IPv6 policy used by the PPP profile.
    pub use_ipv6: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// MPLS policy used by the PPP profile.
    pub use_mpls: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `UPnP` is enabled for the PPP profile.
    pub use_upnp: Option<bool>,
}

/// Response row from `/radius/incoming/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RadiusIncoming {
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether incoming requests are accepted.
    pub accept: Option<bool>,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use alloc::vec;

    use super::*;
    use crate::Row;

    #[test]
    fn certificate_deserializes_chr_row() {
        let mut row = Row::new();
        row.insert(".id".to_string(), "*1".to_string());
        row.insert("common-name".to_string(), "scenario-api".to_string());
        row.insert("crl".to_string(), "false".to_string());
        row.insert("days-valid".to_string(), "365".to_string());
        row.insert("key-size".to_string(), "2048".to_string());
        row.insert("key-type".to_string(), "rsa".to_string());
        row.insert("key-usage".to_string(), "tls-server".to_string());
        row.insert("name".to_string(), "scenario-api".to_string());
        row.insert("private-key".to_string(), "false".to_string());
        row.insert("trust-store".to_string(), "all".to_string());

        let certificate: Certificate = crate::deserialize(&row).expect("certificate row should decode");

        assert_eq!(certificate.id.as_ref().map(ToString::to_string).as_deref(), Some("*1"));
        assert_eq!(certificate.name.as_deref(), Some("scenario-api"));
        assert_eq!(certificate.key_usage, vec!["tls-server".to_string()]);
        assert_eq!(certificate.key_size.as_deref(), Some("2048"));
        assert_eq!(certificate.days_valid, Some(365));
        assert_eq!(certificate.crl, Some(false));
        assert_eq!(certificate.private_key, Some(false));
        assert_eq!(certificate.trust_store.as_deref(), Some("all"));
    }

    #[test]
    fn certificate_deserializes_ec_curve_key_size() {
        let mut row = Row::new();
        row.insert(".id".to_string(), "*3".to_string());
        row.insert("common-name".to_string(), "cn_zwgtls5KrSfNNi2y".to_string());
        row.insert("crl".to_string(), "false".to_string());
        row.insert("days-valid".to_string(), "3650".to_string());
        row.insert("key-size".to_string(), "prime256v1".to_string());
        row.insert("key-type".to_string(), "ec".to_string());
        row.insert("key-usage".to_string(), "key-cert-sign,crl-sign".to_string());
        row.insert("name".to_string(), "ca.crt_0".to_string());

        let certificate: Certificate = crate::deserialize(&row).expect("certificate row should decode");

        assert_eq!(certificate.key_type.as_deref(), Some("ec"));
        assert_eq!(certificate.key_size.as_deref(), Some("prime256v1"));
        assert_eq!(
            certificate.key_usage,
            vec!["key-cert-sign".to_string(), "crl-sign".to_string()]
        );
    }
}
