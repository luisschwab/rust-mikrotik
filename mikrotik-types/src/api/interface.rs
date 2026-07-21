//! Interface and layer-2 API response rows.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::BridgePortStatus;
use crate::primitives::interface::InterfaceName;
use crate::primitives::interface::InterfaceType;
use crate::primitives::interface::Mtu;
use crate::primitives::ip::IpPrefix;
use crate::primitives::ip::MacAddress;
use crate::primitives::system::RouterOsDateTime;
use crate::primitives::system::RouterOsDuration;

/// Response row from `/interface/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Interface {
    /// Internal `RouterOS` row id, when returned.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Interface name.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub name: Option<InterfaceName>,
    /// Default interface name.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub default_name: Option<InterfaceName>,
    /// Interface type, for example `ether`, `bridge`, `vlan`, or `wg`.
    #[serde(rename = "type")]
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface_type: Option<InterfaceType>,
    /// Interface MAC address.
    pub mac_address: Option<MacAddress>,
    /// Configured MTU. `RouterOS` may return numeric values or `auto`.
    pub mtu: Option<Mtu>,
    /// Actual MTU.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub actual_mtu: Option<u32>,
    /// Layer-2 MTU.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub l2mtu: Option<u32>,
    /// Maximum layer-2 MTU.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub max_l2mtu: Option<u32>,
    /// Whether the interface is running.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub running: Option<bool>,
    /// Whether the interface is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Interface comment.
    pub comment: Option<String>,
    /// Number of link-down events.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub link_downs: Option<u64>,
    /// Last link-up timestamp as local `RouterOS` date/time.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub last_link_up_time: Option<RouterOsDateTime>,
    /// Last link-down timestamp as local `RouterOS` date/time.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub last_link_down_time: Option<RouterOsDateTime>,
    /// Whether this interface is a slave of another interface.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub slave: Option<bool>,
    /// Received bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub rx_byte: Option<u64>,
    /// Received packets.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub rx_packet: Option<u64>,
    /// Receive drops.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub rx_drop: Option<u64>,
    /// Receive errors.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub rx_error: Option<u64>,
    /// Transmitted bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_byte: Option<u64>,
    /// Transmitted packets.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_packet: Option<u64>,
    /// Transmit drops.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_drop: Option<u64>,
    /// Transmit errors.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_error: Option<u64>,
    /// Transmit queue drops.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_queue_drop: Option<u64>,
    /// Fast-path received bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub fp_rx_byte: Option<u64>,
    /// Fast-path received packets.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub fp_rx_packet: Option<u64>,
    /// Fast-path transmitted bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub fp_tx_byte: Option<u64>,
    /// Fast-path transmitted packets.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub fp_tx_packet: Option<u64>,
}

/// Response row from `/interface/wireless/registration-table/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WirelessRegistration {
    /// Internal `RouterOS` row ID, when returned.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Wireless interface on which the peer is registered.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// MAC address of the registered peer.
    pub mac_address: Option<MacAddress>,
    /// Time elapsed since the peer associated with the access point.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub uptime: Option<RouterOsDuration>,
    /// Time elapsed since the last peer transmit or receive activity.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub last_activity: Option<RouterOsDuration>,
    /// Whether 802.1X permits data exchange with the peer.
    #[serde(rename = "802.1x-port-enabled", deserialize_with = "crate::optional_bool")]
    pub dot1x_port_enabled: Option<bool>,
    /// Authentication method used for the peer.
    pub authentication_type: Option<String>,
    /// Average signal strength reported for the peer.
    pub signal_strength: Option<String>,
    /// Current peer receive rate.
    pub rx_rate: Option<String>,
    /// Current peer transmit rate.
    pub tx_rate: Option<String>,
}

/// Response row from `/interface/wifi/registration-table/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WifiRegistration {
    /// Internal `RouterOS` row ID, when returned.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// `WiFi` interface on which the peer is registered.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// MAC address of the registered peer.
    pub mac_address: Option<MacAddress>,
    /// Time elapsed since association.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub uptime: Option<RouterOsDuration>,
    /// Time elapsed since the last peer transmit or receive activity.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub last_activity: Option<RouterOsDuration>,
    /// Whether the peer has successfully authenticated.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub authorized: Option<bool>,
    /// Authentication method used for the peer.
    pub auth_type: Option<String>,
    /// Signal strength received from the peer, in dBm.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub signal: Option<i16>,
    /// Current peer receive rate.
    pub rx_rate: Option<String>,
    /// Current peer transmit rate.
    pub tx_rate: Option<String>,
    /// Current receive throughput from the peer.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub rx_bits_per_second: Option<u64>,
    /// Current transmit throughput to the peer.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub tx_bits_per_second: Option<u64>,
}

/// Response row from `/interface/bridge/host/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BridgeHost {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Bridge where this MAC address was learned.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub bridge: Option<InterfaceName>,
    /// Bridge member interface where this MAC address was learned.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// Interface reported by `RouterOS` as the egress interface.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub on_interface: Option<InterfaceName>,
    /// Learned MAC address.
    pub mac_address: Option<MacAddress>,
    /// VLAN id, when the bridge VLAN table reports one.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub vid: Option<u16>,
    /// Whether this is a local bridge/device MAC entry.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub local: Option<bool>,
    /// Whether this row is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether this row is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether this row is invalid.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub invalid: Option<bool>,
    /// Whether this row is external.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub external: Option<bool>,
}

/// Response row from `/interface/bridge/port/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BridgePort {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Bridge this port belongs to.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub bridge: Option<InterfaceName>,
    /// Bridge member interface.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// Port comment.
    pub comment: Option<String>,
    /// Port VLAN id.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub pvid: Option<u16>,
    /// Bridge port number.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub port_number: Option<u16>,
    /// Bridge port status.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub status: Option<BridgePortStatus>,
    /// Whether this row is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether this row is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether this port is inactive.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub inactive: Option<bool>,
    /// Whether this port is forwarding.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub forwarding: Option<bool>,
    /// Whether hardware offload is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub hw_offload: Option<bool>,
    /// Whether hardware acceleration is requested.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub hw: Option<bool>,
    /// Whether BPDU guard is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub bpdu_guard: Option<bool>,
    /// Whether broadcast flooding is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub broadcast_flood: Option<bool>,
    /// Whether fast leave is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub fast_leave: Option<bool>,
    /// Whether ingress filtering is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub ingress_filtering: Option<bool>,
    /// Whether the port is learning.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub learning: Option<bool>,
    /// Whether the port is an edge port.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub edge_port: Option<bool>,
    /// Whether edge-port discovery is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub edge_port_discovery: Option<bool>,
    /// Whether the port is point-to-point.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub point_to_point_port: Option<bool>,
    /// Whether the port is sending RSTP.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub sending_rstp: Option<bool>,
    /// Whether restricted role is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub restricted_role: Option<bool>,
    /// Whether restricted TCN is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub restricted_tcn: Option<bool>,
    /// Whether tag stacking is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub tag_stacking: Option<bool>,
    /// Whether the port is trusted.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub trusted: Option<bool>,
    /// Whether unknown multicast flooding is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub unknown_multicast_flood: Option<bool>,
    /// Whether unknown unicast flooding is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub unknown_unicast_flood: Option<bool>,
    /// Frame admission policy.
    pub frame_types: Option<String>,
    /// Edge mode.
    pub edge: Option<String>,
    /// Learn mode.
    pub learn: Option<String>,
    /// Horizon setting.
    pub horizon: Option<String>,
    /// Multicast router mode.
    pub multicast_router: Option<String>,
    /// Point-to-point mode.
    pub point_to_point: Option<String>,
    /// Port priority.
    pub priority: Option<String>,
    /// STP role.
    pub role: Option<String>,
    /// MVRP applicant state.
    pub mvrp_applicant_state: Option<String>,
    /// MVRP registrar state.
    pub mvrp_registrar_state: Option<String>,
    /// External FDB status.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub external_fdb_status: Option<bool>,
    /// Auto-isolate status.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub auto_isolate: Option<bool>,
}

/// Response row from `/interface/bridge/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Bridge {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this bridge.
    pub name: Option<InterfaceName>,
    /// Comment configured on this bridge.
    pub comment: Option<String>,
    /// ARP handling mode.
    pub arp: Option<String>,
    /// Timeout used for learned ARP entries.
    pub arp_timeout: Option<String>,
    /// `EtherType` used for bridge VLAN filtering.
    pub ether_type: Option<String>,
    /// Bridge frame admission policy.
    pub frame_types: Option<String>,
    /// Bridge port cost calculation mode.
    pub port_cost_mode: Option<String>,
    /// Priority value used by the entry.
    pub priority: Option<String>,
    /// Spanning tree protocol mode used by the bridge.
    pub protocol_mode: Option<String>,
    /// Configured MTU.
    pub mtu: Option<Mtu>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Actual MTU currently in use.
    pub actual_mtu: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Layer-2 MTU.
    pub l2mtu: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port VLAN ID.
    pub pvid: Option<u16>,
    /// MAC address associated with this row.
    pub mac_address: Option<MacAddress>,
    /// Administrative MAC address configured for the bridge.
    pub admin_mac: Option<MacAddress>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` automatically selects the bridge MAC address.
    pub auto_mac: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DHCP snooping is enabled on the bridge.
    pub dhcp_snooping: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridge fast-forward is enabled.
    pub fast_forward: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IGMP snooping is enabled.
    pub igmp_snooping: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether ingress VLAN filtering is enabled.
    pub ingress_filtering: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether MVRP is enabled.
    pub mvrp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridge VLAN filtering is enabled.
    pub vlan_filtering: Option<bool>,
}

/// Response row from `/interface/bridge/vlan/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BridgeVlan {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bridge interface associated with this row.
    pub bridge: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Tagged member interfaces.
    pub tagged: Vec<InterfaceName>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Untagged member interfaces.
    pub untagged: Vec<InterfaceName>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Currently active tagged member interfaces.
    pub current_tagged: Vec<InterfaceName>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Currently active untagged member interfaces.
    pub current_untagged: Vec<InterfaceName>,
    /// VLAN ID set configured on this row.
    pub vlan_ids: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/interface/list/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InterfaceList {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this interface list.
    pub name: Option<String>,
    /// Comment configured on this interface list.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is built in to `RouterOS`.
    pub builtin: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/interface/list/member/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct InterfaceListMember {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// List name.
    pub list: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/interface/vlan/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct VlanInterface {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this vlan interface.
    pub name: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    /// Comment configured on this vlan interface.
    pub comment: Option<String>,
    /// ARP handling mode.
    pub arp: Option<String>,
    /// Timeout used for learned ARP entries.
    pub arp_timeout: Option<String>,
    /// Loop protection mode.
    pub loop_protect: Option<String>,
    /// Duration an interface stays disabled after loop detection.
    pub loop_protect_disable_time: Option<String>,
    /// Interval between loop-protection probe packets.
    pub loop_protect_send_interval: Option<String>,
    /// Current loop-protection status.
    pub loop_protect_status: Option<String>,
    /// MAC address associated with this row.
    pub mac_address: Option<MacAddress>,
    /// Configured MTU.
    pub mtu: Option<Mtu>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Layer-2 MTU.
    pub l2mtu: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// VLAN ID.
    pub vlan_id: Option<u16>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether MVRP is enabled.
    pub mvrp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the VLAN interface uses the 802.1ad service tag.
    pub use_service_tag: Option<bool>,
}

/// Response row from `/interface/wireguard/print`.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WireGuardInterface {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this wire guard interface.
    pub name: Option<InterfaceName>,
    /// Comment configured on this wire guard interface.
    pub comment: Option<String>,
    /// Private key value.
    #[serde(skip_serializing)]
    pub private_key: Option<String>,
    /// Public key value.
    pub public_key: Option<String>,
    /// Configured MTU.
    pub mtu: Option<Mtu>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub listen_port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
}

impl fmt::Debug for WireGuardInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WireGuardInterface")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("comment", &self.comment)
            .field("private_key", &self.private_key.as_ref().map(|_| "<redacted>"))
            .field("public_key", &self.public_key)
            .field("mtu", &self.mtu)
            .field("listen_port", &self.listen_port)
            .field("disabled", &self.disabled)
            .field("running", &self.running)
            .finish()
    }
}

/// Response row from `/interface/wireguard/peers/print`.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WireGuardPeer {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this wire guard peer.
    pub name: Option<String>,
    /// Comment configured on this wire guard peer.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// `WireGuard` peer allowed-address prefixes.
    pub allowed_address: Vec<IpPrefix>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Addresses assigned or advertised for the client side.
    pub client_address: Vec<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current endpoint address.
    pub current_endpoint_address: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub current_endpoint_port: Option<u16>,
    /// Configured endpoint address.
    pub endpoint_address: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub endpoint_port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Time elapsed since the last handshake.
    pub last_handshake: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Persistent keepalive interval.
    pub persistent_keepalive: Option<RouterOsDuration>,
    /// Public key value.
    pub public_key: Option<String>,
    /// Preshared key value.
    #[serde(skip_serializing)]
    pub preshared_key: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Received byte count.
    pub rx: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Transmitted byte count.
    pub tx: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

impl fmt::Debug for WireGuardPeer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WireGuardPeer")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("comment", &self.comment)
            .field("interface", &self.interface)
            .field("allowed_address", &self.allowed_address)
            .field("client_address", &self.client_address)
            .field("current_endpoint_address", &self.current_endpoint_address)
            .field("current_endpoint_port", &self.current_endpoint_port)
            .field("endpoint_address", &self.endpoint_address)
            .field("endpoint_port", &self.endpoint_port)
            .field("last_handshake", &self.last_handshake)
            .field("persistent_keepalive", &self.persistent_keepalive)
            .field("public_key", &self.public_key)
            .field("preshared_key", &self.preshared_key.as_ref().map(|_| "<redacted>"))
            .field("rx", &self.rx)
            .field("tx", &self.tx)
            .field("disabled", &self.disabled)
            .field("dynamic", &self.dynamic)
            .finish()
    }
}

/// Response row from `/interface/bridge/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct BridgeSettings {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether fast path forwarding is allowed.
    pub allow_fast_path: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridge fast path is currently active.
    pub bridge_fast_path_active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridged traffic is passed through the IP firewall.
    pub use_ip_firewall: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridged `PPPoE` traffic is passed through the IP firewall.
    pub use_ip_firewall_for_pppoe: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether bridged VLAN traffic is passed through the IP firewall.
    pub use_ip_firewall_for_vlan: Option<bool>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes forwarded through bridge fast-forward.
    pub bridge_fast_forward_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets forwarded through bridge fast-forward.
    pub bridge_fast_forward_packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes forwarded through bridge fast path.
    pub bridge_fast_path_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets forwarded through bridge fast path.
    pub bridge_fast_path_packets: Option<u64>,
}

/// Response row from `/interface/detect-internet/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DetectInternet {
    /// Interface list used by Detect Internet probing.
    pub detect_interface_list: Option<String>,
    /// Interface list classified as Internet-facing.
    pub internet_interface_list: Option<String>,
    /// Interface list classified as LAN-facing.
    pub lan_interface_list: Option<String>,
    /// Interface list classified as WAN-facing.
    pub wan_interface_list: Option<String>,
}

/// Response row from `/interface/ethernet/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EthernetInterface {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this ethernet interface.
    pub name: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Factory default interface name.
    pub default_name: Option<InterfaceName>,
    /// Comment configured on this ethernet interface.
    pub comment: Option<String>,
    /// Advertised addresses or link capabilities.
    pub advertise: Option<String>,
    /// ARP handling mode.
    pub arp: Option<String>,
    /// Timeout used for learned ARP entries.
    pub arp_timeout: Option<String>,
    /// Configured interface bandwidth setting.
    pub bandwidth: Option<String>,
    /// Loop protection mode.
    pub loop_protect: Option<String>,
    /// Current loop-protection status.
    pub loop_protect_status: Option<String>,
    /// Receive flow-control mode.
    pub rx_flow_control: Option<String>,
    /// Transmit flow-control mode.
    pub tx_flow_control: Option<String>,
    /// Switch chip or switch group associated with the interface.
    pub switch: Option<String>,
    /// MAC address associated with this row.
    pub mac_address: Option<MacAddress>,
    /// Original hardware MAC address.
    pub orig_mac_address: Option<MacAddress>,
    /// Configured MTU.
    pub mtu: Option<Mtu>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Layer-2 MTU.
    pub l2mtu: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Duration an interface stays disabled after loop detection.
    pub loop_protect_disable_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between loop-protection probe packets.
    pub loop_protect_send_interval: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether Ethernet link autonegotiation is enabled.
    pub auto_negotiation: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the Ethernet interface is a slave interface.
    pub slave: Option<bool>,
    #[serde(flatten)]
    /// Whether hardware counters are enabled or available.
    pub counters: EthernetCounters,
}

/// Ethernet packet and byte counters shared by Ethernet menus.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EthernetCounters {
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Bytes received by the Ethernet driver.
    pub driver_rx_byte: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received packet count.
    pub driver_rx_packet: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Bytes transmitted by the Ethernet driver.
    pub driver_tx_byte: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmitted packet count.
    pub driver_tx_packet: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received byte count.
    pub rx_bytes: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmitted byte count.
    pub tx_bytes: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received broadcast Ethernet frames.
    pub rx_broadcast: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmitted broadcast Ethernet frames.
    pub tx_broadcast: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received multicast Ethernet frames.
    pub rx_multicast: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmitted multicast Ethernet frames.
    pub tx_multicast: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received Ethernet frames with FCS errors.
    pub rx_fcs_error: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Received Ethernet frames with alignment errors.
    pub rx_align_error: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Receive overflow counter.
    pub rx_overflow: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmit collision counter.
    pub tx_collision: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Excessive transmit collision counter.
    pub tx_excessive_collision: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Late transmit collision counter.
    pub tx_late_collision: Vec<u64>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Transmit underrun counter.
    pub tx_underrun: Vec<u64>,
}

/// Response row from `/interface/ethernet/switch/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EthernetSwitch {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this ethernet switch.
    pub name: Option<String>,
    #[serde(rename = "type")]
    /// Switch chip type.
    pub switch_type: Option<String>,
    /// Switch port mirrored as the traffic source.
    pub mirror_source: Option<String>,
    /// Switch port receiving mirrored traffic.
    pub mirror_target: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether switch CPU-port flow control is enabled.
    pub cpu_flow_control: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(flatten)]
    /// Whether hardware counters are enabled or available.
    pub counters: EthernetCounters,
}

/// Response row from `/interface/ethernet/switch/port/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EthernetSwitchPort {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this ethernet switch port.
    pub name: Option<InterfaceName>,
    /// Switch chip or switch group associated with the interface.
    pub switch: Option<String>,
    /// VLAN header handling mode for the switch port.
    pub vlan_header: Option<String>,
    /// VLAN mode configured for the switch port.
    pub vlan_mode: Option<String>,
    /// Default VLAN ID assigned to untagged traffic.
    pub default_vlan_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this interface or service is running.
    pub running: Option<bool>,
    #[serde(flatten)]
    /// Whether hardware counters are enabled or available.
    pub counters: EthernetCounters,
}

/// Response row from `/interface/ethernet/switch/port-isolation/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct EthernetSwitchPortIsolation {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Name of this ethernet switch port isolation.
    pub name: Option<InterfaceName>,
    /// Switch chip or switch group associated with the interface.
    pub switch: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
}

/// Response row from `/interface/lte/apn/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LteApn {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this lte apn.
    pub name: Option<String>,
    /// Access point name used by the LTE profile.
    pub apn: Option<String>,
    /// Authentication method used by this entry.
    pub authentication: Option<String>,
    /// IP stack mode requested by the LTE profile.
    pub ip_type: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Administrative distance for installed default routes.
    pub default_route_distance: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether a default route is installed from this configuration.
    pub add_default_route: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the LTE modem network-provided APN is used.
    pub use_network_apn: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DNS servers learned from the peer are used.
    pub use_peer_dns: Option<bool>,
}

/// Response row from `/interface/wireless/security-profiles/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct WirelessSecurityProfile {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this wireless security profile.
    pub name: Option<String>,
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
    /// Wireless authentication methods allowed by the profile.
    pub authentication_types: Option<String>,
    /// EAP methods allowed by the wireless security profile.
    pub eap_methods: Option<String>,
    /// Group ciphers allowed by the wireless security profile.
    pub group_ciphers: Option<String>,
    /// Unicast ciphers allowed by the wireless security profile.
    pub unicast_ciphers: Option<String>,
    /// Supplicant identity used by wireless authentication.
    pub supplicant_identity: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Wireless group-key rotation interval.
    pub group_key_update: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between interim accounting updates.
    pub interim_update: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether PMKID is disabled for the wireless profile.
    pub disable_pmkid: Option<bool>,
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString as _;
    use alloc::vec;

    use super::BridgePort;
    use super::EthernetSwitch;
    use super::EthernetSwitchPort;
    use super::Interface;
    use super::WifiRegistration;
    use super::WireGuardInterface;
    use super::WireGuardPeer;
    use super::WirelessRegistration;
    use crate::Row;
    use crate::primitives::interface::InterfaceName;

    #[test]
    fn interface_deserializes_typed_link_timestamps() {
        let mut row = Row::new();
        row.insert(".id".into(), "*2".into());
        row.insert("name".into(), "ether1".into());
        row.insert("max-l2mtu".into(), "2028".into());
        row.insert("last-link-up-time".into(), "2026-06-04 18:49:35".into());
        row.insert("last-link-down-time".into(), "2026-06-04 18:51:39".into());

        let interface = crate::deserialize::<Interface>(&row).expect("interface row should deserialize");

        assert_eq!(
            interface
                .last_link_up_time
                .expect("link up timestamp should be present")
                .to_string(),
            "2026-06-04 18:49:35"
        );
        assert_eq!(
            interface
                .last_link_down_time
                .expect("link down timestamp should be present")
                .to_string(),
            "2026-06-04 18:51:39"
        );
        assert_eq!(interface.max_l2mtu, Some(2028));
    }

    #[test]
    fn legacy_wireless_registration_deserializes_routeros_v6_and_v7_variants() {
        let mut v6_row = Row::new();
        v6_row.insert(".id".into(), "*7".into());
        v6_row.insert("interface".into(), "wlan1".into());
        v6_row.insert("mac-address".into(), "00:11:22:33:44:55".into());
        v6_row.insert("uptime".into(), "1d2h3m4s".into());
        v6_row.insert("last-activity".into(), "120ms".into());
        v6_row.insert("802.1x-port-enabled".into(), "false".into());
        v6_row.insert("authentication-type".into(), "wpa2-psk".into());
        v6_row.insert("signal-strength".into(), "-64dBm@6Mbps".into());
        v6_row.insert("rx-rate".into(), "54Mbps".into());
        v6_row.insert("tx-rate".into(), "48Mbps".into());

        let registration =
            crate::deserialize::<WirelessRegistration>(&v6_row).expect("v6 legacy registration should deserialize");

        assert_eq!(
            registration.interface.as_ref().map(InterfaceName::as_str),
            Some("wlan1")
        );
        assert_eq!(
            registration.mac_address.map(|address| address.to_string()).as_deref(),
            Some("00:11:22:33:44:55")
        );
        assert_eq!(registration.dot1x_port_enabled, Some(false));
        assert_eq!(registration.signal_strength.as_deref(), Some("-64dBm@6Mbps"));
        assert_eq!(registration.rx_rate.as_deref(), Some("54Mbps"));

        let mut v7_row = Row::new();
        v7_row.insert("interface".into(), "wlan-backhaul".into());
        v7_row.insert("mac-address".into(), "00:11:22:33:44:66".into());
        v7_row.insert("signal-strength".into(), "-59".into());

        let registration =
            crate::deserialize::<WirelessRegistration>(&v7_row).expect("v7 legacy registration should deserialize");

        assert_eq!(registration.dot1x_port_enabled, None);
        assert_eq!(registration.uptime, None);
        assert_eq!(registration.signal_strength.as_deref(), Some("-59"));
    }

    #[test]
    fn wifi_registration_deserializes_routeros_v7_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*3".into());
        row.insert("interface".into(), "wifi1".into());
        row.insert("mac-address".into(), "00:AA:BB:CC:DD:EE".into());
        row.insert("uptime".into(), "6h24m21s".into());
        row.insert("last-activity".into(), "0ms".into());
        row.insert("authorized".into(), "true".into());
        row.insert("auth-type".into(), "wpa2-psk".into());
        row.insert("signal".into(), "-72".into());
        row.insert("rx-rate".into(), "360.3Mbps".into());
        row.insert("tx-rate".into(), "432.3Mbps".into());
        row.insert("rx-bits-per-second".into(), "123456".into());
        row.insert("tx-bits-per-second".into(), "654321".into());

        let registration = crate::deserialize::<WifiRegistration>(&row).expect("WiFi registration should deserialize");

        assert_eq!(
            registration.interface.as_ref().map(InterfaceName::as_str),
            Some("wifi1")
        );
        assert_eq!(registration.authorized, Some(true));
        assert_eq!(registration.signal, Some(-72));
        assert_eq!(registration.rx_bits_per_second, Some(123_456));
        assert_eq!(registration.tx_bits_per_second, Some(654_321));
    }

    #[test]
    fn bridge_port_deserializes_live_scalar_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*0".into());
        row.insert("interface".into(), "ether2".into());
        row.insert("bridge".into(), "BRIDGE_VLAN".into());
        row.insert("pvid".into(), "10".into());
        row.insert("port-number".into(), "1".into());
        row.insert("status".into(), "in-bridge".into());
        row.insert("bpdu-guard".into(), "false".into());
        row.insert("broadcast-flood".into(), "true".into());
        row.insert("edge-port".into(), "true".into());
        row.insert("ingress-filtering".into(), "true".into());
        row.insert("learning".into(), "true".into());
        row.insert("restricted-role".into(), "false".into());
        row.insert("unknown-unicast-flood".into(), "true".into());
        row.insert("frame-types".into(), "admit-only-untagged-and-priority-tagged".into());
        row.insert("role".into(), "designated-port".into());

        let port = crate::deserialize::<BridgePort>(&row).expect("bridge port row should deserialize");

        assert_eq!(port.pvid, Some(10));
        assert_eq!(port.port_number, Some(1));
        assert_eq!(port.bpdu_guard, Some(false));
        assert_eq!(port.broadcast_flood, Some(true));
        assert_eq!(port.edge_port, Some(true));
        assert_eq!(port.ingress_filtering, Some(true));
        assert_eq!(port.learning, Some(true));
        assert_eq!(port.restricted_role, Some(false));
        assert_eq!(port.unknown_unicast_flood, Some(true));
        assert_eq!(
            port.frame_types.as_deref(),
            Some("admit-only-untagged-and-priority-tagged")
        );
        assert_eq!(port.role.as_deref(), Some("designated-port"));
    }

    #[test]
    fn ethernet_switch_deserializes_comma_list_counters() {
        let mut row = Row::new();
        row.insert(".id".into(), "*0".into());
        row.insert("name".into(), "switch1".into());
        row.insert("type".into(), "MediaTek-MT7621".into());
        row.insert("driver-rx-byte".into(), "11511702323,0".into());
        row.insert("driver-rx-packet".into(), "105007900,0".into());
        row.insert("driver-tx-byte".into(), "448579048,0".into());
        row.insert("driver-tx-packet".into(), "2396222,0".into());
        row.insert("rx-bytes".into(), "157422711,0".into());
        row.insert("tx-bytes".into(), "0,458163936".into());
        row.insert("rx-fcs-error".into(), "0,0".into());
        row.insert("invalid".into(), "false".into());

        let switch = crate::deserialize::<EthernetSwitch>(&row).expect("ethernet switch row should deserialize");

        assert_eq!(switch.switch_type.as_deref(), Some("MediaTek-MT7621"));
        assert_eq!(switch.counters.driver_rx_byte, vec![11_511_702_323, 0]);
        assert_eq!(switch.counters.tx_bytes, vec![0, 458_163_936]);
        assert_eq!(switch.counters.rx_fcs_error, vec![0, 0]);
        assert_eq!(switch.invalid, Some(false));
    }

    #[test]
    fn ethernet_switch_port_deserializes_auto_default_vlan_and_scalar_counters() {
        let mut row = Row::new();
        row.insert(".id".into(), "*1".into());
        row.insert("name".into(), "ether1".into());
        row.insert("switch".into(), "switch1".into());
        row.insert("default-vlan-id".into(), "auto".into());
        row.insert("driver-rx-byte".into(), "39024322928447".into());
        row.insert("rx-bytes".into(), "39218030774158".into());
        row.insert("tx-bytes".into(), "6286125183807".into());
        row.insert("invalid".into(), "false".into());
        row.insert("vlan-header".into(), "leave-as-is".into());
        row.insert("vlan-mode".into(), "disabled".into());

        let port = crate::deserialize::<EthernetSwitchPort>(&row).expect("ethernet switch port row should deserialize");

        assert_eq!(port.default_vlan_id.as_deref(), Some("auto"));
        assert_eq!(port.counters.driver_rx_byte, vec![39_024_322_928_447]);
        assert_eq!(port.counters.rx_bytes, vec![39_218_030_774_158]);
        assert_eq!(port.counters.tx_bytes, vec![6_286_125_183_807]);
        assert_eq!(port.invalid, Some(false));
    }

    #[test]
    fn wireguard_peer_deserializes_live_typed_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*1".into());
        row.insert("interface".into(), "wg-hq".into());
        row.insert("allowed-address".into(), "10.20.50.1/32,::/0".into());
        row.insert(
            "client-address".into(),
            "10.71.108.69/32,fc00:bbbb:bbbb:bb01::8:6c44/128".into(),
        );
        row.insert("current-endpoint-port".into(), "42069".into());
        row.insert("last-handshake".into(), "8s".into());
        row.insert("rx".into(), "3391440792".into());

        let peer = crate::deserialize::<WireGuardPeer>(&row).expect("WireGuard peer should deserialize");

        assert_eq!(peer.allowed_address.len(), 2);
        assert_eq!(peer.client_address.len(), 2);
        assert_eq!(peer.current_endpoint_port, Some(42069));
        assert_eq!(
            peer.last_handshake.expect("handshake should be present").to_string(),
            "8s"
        );
        assert_eq!(peer.rx, Some(3_391_440_792));
    }

    #[test]
    fn wireguard_secrets_are_not_formatted_or_serialized() {
        let interface = WireGuardInterface {
            private_key: Some("private-secret".to_string()),
            ..WireGuardInterface::default()
        };
        let peer = WireGuardPeer {
            preshared_key: Some("preshared-secret".to_string()),
            ..WireGuardPeer::default()
        };

        let debug = format!("{interface:?} {peer:?}");
        let serialized = format!(
            "{} {}",
            serde_json::to_string(&interface).unwrap(),
            serde_json::to_string(&peer).unwrap()
        );

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("private-secret"));
        assert!(!debug.contains("preshared-secret"));
        assert!(!serialized.contains("private-secret"));
        assert!(!serialized.contains("preshared-secret"));
        assert!(!serialized.contains("private-key"));
        assert!(!serialized.contains("preshared-key"));
    }
}
