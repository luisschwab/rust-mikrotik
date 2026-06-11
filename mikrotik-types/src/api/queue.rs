//! Queue endpoint rows.
//!
//! This module covers queue configuration and state rows from `/queue/*`.

use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::IpPrefix;
use crate::primitives::system::RouterOsDuration;

/// Response row from `/queue/interface/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct QueueInterface {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    /// Queue type configured on the interface.
    pub queue: Option<String>,
    /// Queue type currently active on the interface.
    pub active_queue: Option<String>,
    /// Default queue type configured for the interface.
    pub default_queue: Option<String>,
}

/// Response row from `/queue/type/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct QueueType {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this queue type.
    pub name: Option<String>,
    /// Queue implementation kind.
    pub kind: Option<String>,
    /// Classifier used by the PCQ queue.
    pub pcq_classifier: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packet limit for the multi-queue PFIFO queue.
    pub mq_pfifo_limit: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Burst rate configured for the PCQ queue.
    pub pcq_burst_rate: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Burst threshold configured for the PCQ queue.
    pub pcq_burst_threshold: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Burst time configured for the PCQ queue.
    pub pcq_burst_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv4 destination address mask used by PCQ classification.
    pub pcq_dst_address_mask: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv6 destination address mask used by PCQ classification.
    pub pcq_dst_address6_mask: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Per-substream packet limit for the PCQ queue.
    pub pcq_limit: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Rate limit configured for each PCQ substream.
    pub pcq_rate: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv4 source address mask used by PCQ classification.
    pub pcq_src_address_mask: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv6 source address mask used by PCQ classification.
    pub pcq_src_address6_mask: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Total packet limit for the PCQ queue.
    pub pcq_total_limit: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packet limit for the PFIFO queue.
    pub pfifo_limit: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Average packet size used by the RED queue.
    pub red_avg_packet: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Burst allowance used by the RED queue.
    pub red_burst: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packet limit used by the RED queue.
    pub red_limit: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum threshold used by the RED queue.
    pub red_max_threshold: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Minimum threshold used by the RED queue.
    pub red_min_threshold: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packet allotment used by the SFQ queue.
    pub sfq_allot: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Perturb interval used by the SFQ queue.
    pub sfq_perturb: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// IPv6 destination mask values exposed by `RouterOS` for PCQ.
    pub pcq_dst_address6_mask_values: Vec<IpPrefix>,
}
