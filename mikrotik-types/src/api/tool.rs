//! Tool endpoint rows.
//!
//! This module covers operational helper menus from `/tool/*`.

use alloc::string::String;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::InterfaceName;
use crate::primitives::system::RouterOsByteSize;
use crate::primitives::system::RouterOsDuration;

/// Response row from `/tool/bandwidth-server/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BandwidthServer {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// First UDP port allocated by the bandwidth-test server.
    pub allocate_udp_ports_from: Option<u16>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum concurrent sessions accepted by the service.
    pub max_sessions: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether clients must authenticate before using the service.
    pub authenticate: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/tool/e-mail/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Email {
    /// Sender address used for outgoing email.
    pub from: Option<String>,
    /// Server name or address.
    pub server: Option<String>,
    /// TLS mode used for outgoing email.
    pub tls: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub port: Option<u16>,
}

/// Response row from `/tool/graphing/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Graphing {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Refresh interval for graphing pages.
    pub page_refresh: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Sampling interval used by graphing storage.
    pub store_every: Option<RouterOsDuration>,
}

/// Response row from `/tool/mac-server/print` and related allowed-list menus.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MacServer {
    /// Interface list allowed to use the MAC server.
    pub allowed_interface_list: Option<String>,
}

/// Response row from `/tool/mac-server/ping/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MacServerPing {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/tool/romon/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Romon {
    /// Internal `RouterOS` row ID.
    pub id: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/tool/romon/port/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RomonPort {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// `RoMON` path cost for this port.
    pub cost: Option<u32>,
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

/// Response row from `/tool/sniffer/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Sniffer {
    /// Packet direction matched by the sniffer filter.
    pub filter_direction: Option<String>,
    /// Boolean operator used between sniffer filter entries.
    pub filter_operator_between_entries: Option<String>,
    /// Packet stream selected by the sniffer filter.
    pub filter_stream: Option<String>,
    /// Whether sniffer quick view includes frame data.
    pub quick_show_frame: Option<String>,
    /// Remote server that receives streamed sniffer packets.
    pub streaming_server: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum capture file size used by the sniffer.
    pub file_limit: Option<RouterOsByteSize>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Memory limit used by the sniffer.
    pub memory_limit: Option<RouterOsByteSize>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the sniffer memory buffer wraps when full.
    pub memory_scroll: Option<bool>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of rows shown by sniffer quick view.
    pub quick_rows: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the sniffer captures packet headers only.
    pub only_headers: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether packet streaming is enabled for the sniffer.
    pub streaming_enabled: Option<bool>,
}

/// Response row from `/tool/sms/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Sms {
    /// Update channel selected for package updates.
    pub channel: Option<String>,
    /// Port number.
    pub port: Option<String>,
    /// Storage location used for SMS messages.
    pub sms_storage: Option<String>,
    /// Current SMS subsystem status.
    pub status: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether SMS receiving is enabled.
    pub receive_enabled: Option<bool>,
}

/// Response row from `/tool/traffic-generator/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct TrafficGenerator {
    /// Identifier assigned to the traffic-generator test.
    pub test_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum latency bucket tracked by the traffic generator.
    pub latency_distribution_max: Option<RouterOsDuration>,
    /// Interval used for traffic-generator latency measurements.
    pub latency_distribution_measure_interval: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of latency distribution samples retained.
    pub latency_distribution_samples: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of traffic-generator statistics samples retained.
    pub stats_samples_to_keep: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether out-of-order packets are measured.
    pub measure_out_of_order: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
}

/// Response row from `/tool/traffic-generator/stats/latency-distribution/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct TrafficGeneratorLatencyDistribution {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Counter value for this statistic row.
    pub count: Option<u64>,
    /// Latency bucket represented by this traffic-generator row.
    pub latency: Option<String>,
    /// Share of samples that fell into this latency bucket.
    pub share: Option<String>,
}
