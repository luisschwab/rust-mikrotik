//! IPv4, IPv6, firewall, neighbor, and DHCP API response rows.

use alloc::string::String;
use alloc::vec::Vec;
use core::net::IpAddr;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::ArpStatus;
use crate::primitives::ip::DhcpLeaseStatus;
use crate::primitives::ip::DiscoveryProtocol;
use crate::primitives::ip::IpEndpointAddress;
use crate::primitives::ip::IpPrefix;
use crate::primitives::ip::MacAddress;
use crate::primitives::ip::ScopedIpAddress;
use crate::primitives::ip::SystemCapability;
use crate::primitives::routing::RouteGateway;
use crate::primitives::routing::RoutingTableName;
use crate::primitives::system::RouterOsByteSize;
use crate::primitives::system::RouterOsDateTime;
use crate::primitives::system::RouterOsDuration;
use crate::primitives::system::RouterOsVersion;

/// Response row from `/ip/neighbor/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Neighbor {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Local interface where the neighbor was seen.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// Remote interface name reported by LLDP/CDP/MNDP.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface_name: Option<InterfaceName>,
    /// Neighbor management address.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address: Option<IpAddr>,
    /// IPv4 neighbor address.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address4: Option<IpAddr>,
    /// IPv6 neighbor address.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address6: Option<IpAddr>,
    /// Neighbor MAC address.
    pub mac_address: Option<MacAddress>,
    /// Neighbor identity or hostname.
    pub identity: Option<String>,
    /// Neighbor platform.
    pub platform: Option<String>,
    /// Neighbor `RouterOS` version, when reported.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub version: Option<RouterOsVersion>,
    /// Neighbor board, when reported.
    pub board: Option<String>,
    /// `MikroTik` software id, when reported.
    pub software_id: Option<String>,
    /// Neighbor uptime, when reported.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub uptime: Option<RouterOsDuration>,
    /// Neighbor age.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub age: Option<RouterOsDuration>,
    /// Whether the neighbor row includes IPv6.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub ipv6: Option<bool>,
    /// LLDP system capabilities.
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    pub system_caps: Vec<SystemCapability>,
    /// Enabled LLDP system capabilities.
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    pub system_caps_enabled: Vec<SystemCapability>,
    /// LLDP system description.
    pub system_description: Option<String>,
    /// Whether neighbor discovery unpacks advertised details.
    pub unpack: Option<String>,
    /// Discovery protocols that reported this neighbor.
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    pub discovered_by: Vec<DiscoveryProtocol>,
}

impl Neighbor {
    /// Returns a usable management address, filtering `RouterOS` placeholder values.
    #[must_use]
    pub fn management_address(&self) -> Option<IpAddr> {
        self.address.filter(|address| !address.is_unspecified())
    }

    /// Whether this neighbor looks like a `MikroTik` `RouterOS` device.
    #[must_use]
    pub fn is_mikrotik(&self) -> bool {
        self.board.is_some()
            || self
                .version
                .as_ref()
                .is_some_and(|version| version.as_str().contains("(stable)") || version.as_str().contains("RouterOS"))
    }
}

/// Response row from `/ip/address/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Address {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Address with prefix length.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address: Option<IpPrefix>,
    /// Network address.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub network: Option<IpAddr>,
    /// Configured interface.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// Actual interface.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub actual_interface: Option<InterfaceName>,
    /// Whether the address is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether the address is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether the address is invalid.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub invalid: Option<bool>,
    /// Address comment.
    pub comment: Option<String>,
}

/// Response row from `/ip/route/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Route {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Destination prefix.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub dst_address: Option<IpPrefix>,
    /// Gateway value.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub gateway: Option<RouteGateway>,
    /// Immediate gateway value.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub immediate_gw: Option<RouteGateway>,
    /// Routing table.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub routing_table: Option<RoutingTableName>,
    /// Local address for connected routes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub local_address: Option<ScopedIpAddress>,
    /// VRF interface, when reported.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub vrf_interface: Option<InterfaceName>,
    /// Administrative distance.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub distance: Option<u32>,
    /// Scope.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub scope: Option<u32>,
    /// Target scope.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub target_scope: Option<u32>,
    /// Whether the route is active.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub active: Option<bool>,
    /// Whether the route is inactive.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub inactive: Option<bool>,
    /// Whether the route is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether the route is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether the route is static.
    #[serde(rename = "static", deserialize_with = "crate::optional_bool")]
    pub static_route: Option<bool>,
    /// Whether the route is connected.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub connect: Option<bool>,
    /// Whether the route came from DHCP.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dhcp: Option<bool>,
    /// Whether ECMP is active.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub ecmp: Option<bool>,
    /// Whether the route is hardware offloaded.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub hw_offloaded: Option<bool>,
    /// Whether hardware offload is suppressed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub suppress_hw_offload: Option<bool>,
    /// Route comment.
    pub comment: Option<String>,
}

/// Response row from `/ip/arp/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ArpEntry {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// IP address associated with the ARP entry.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address: Option<IpAddr>,
    /// MAC address associated with the ARP entry.
    pub mac_address: Option<MacAddress>,
    /// Hostname associated with the ARP entry, when available.
    pub host_name: Option<String>,
    /// Interface where `RouterOS` sees this entry.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub interface: Option<InterfaceName>,
    /// ARP status.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub status: Option<ArpStatus>,
    /// Whether the ARP entry is complete.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub complete: Option<bool>,
    /// Whether the ARP entry came from DHCP.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dhcp: Option<bool>,
    /// Whether the ARP entry is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether the ARP entry is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether the ARP entry is invalid.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub invalid: Option<bool>,
    /// Whether the ARP entry is published.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub published: Option<bool>,
}

/// Response row from `/ip/dhcp-server/lease/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DhcpLease {
    /// Internal `RouterOS` row id.
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    pub id: Option<RouterOsId>,
    /// Configured or leased address.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub address: Option<IpAddr>,
    /// Active address, when bound.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub active_address: Option<IpAddr>,
    /// Configured or leased MAC address.
    pub mac_address: Option<MacAddress>,
    /// Active MAC address, when bound.
    pub active_mac_address: Option<MacAddress>,
    /// DHCP lease hostname, when the client provides one.
    pub host_name: Option<String>,
    /// DHCP server name.
    pub server: Option<String>,
    /// Active DHCP server name.
    pub active_server: Option<String>,
    /// DHCP lease status.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub status: Option<DhcpLeaseStatus>,
    /// Client id.
    pub client_id: Option<String>,
    /// Active client id.
    pub active_client_id: Option<String>,
    /// Lease age.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub age: Option<RouterOsDuration>,
    /// Expiration duration.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub expires_after: Option<RouterOsDuration>,
    /// Last seen duration.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub last_seen: Option<RouterOsDuration>,
    /// Whether the lease is blocked.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub blocked: Option<bool>,
    /// Whether the lease is disabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub disabled: Option<bool>,
    /// Whether the lease is dynamic.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dynamic: Option<bool>,
    /// Whether the lease came from RADIUS.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub radius: Option<bool>,
}

/// Response row from `/ip/dhcp-client/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DhcpClient {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Address assigned to the DHCP client.
    pub address: Option<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Configured gateway value.
    pub gateway: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// DHCP server address.
    pub dhcp_server: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Primary DNS server learned by the DHCP client.
    pub primary_dns: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Secondary DNS server learned by the DHCP client.
    pub secondary_dns: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Remaining time before this row expires.
    pub expires_after: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Administrative distance for installed default routes.
    pub default_route_distance: Option<u32>,
    /// Current DHCP client status.
    pub status: Option<String>,
    /// Comment configured on this dhcp client.
    pub comment: Option<String>,
    /// DHCP options requested or applied by the client.
    pub dhcp_options: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether a default route is installed from this configuration.
    pub add_default_route: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DNS servers learned from the peer are used.
    pub use_peer_dns: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether NTP servers learned from the peer are used.
    pub use_peer_ntp: Option<bool>,
}

/// Response row from `/ip/dhcp-server/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DhcpServer {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this dhcp server.
    pub name: Option<String>,
    /// Address pool used to allocate client addresses.
    pub address_pool: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// DHCP lease duration handed out by the server.
    pub lease_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DHCP leases create ARP entries.
    pub add_arp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether RADIUS integration is enabled for this row.
    pub use_radius: Option<bool>,
}

/// Response row from `/ip/dhcp-server/network/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DhcpServerNetwork {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Network prefix served by the DHCP network entry.
    pub address: Option<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Configured gateway value.
    pub gateway: Option<IpAddr>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// DNS servers advertised by the DHCP network.
    pub dns_server: Vec<IpAddr>,
    /// Comment configured on this dhcp server network.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/ip/dns/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Dns {
    #[serde(deserialize_with = "crate::comma_list")]
    /// DNS upstream servers configured on the router.
    pub servers: Vec<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Configured DNS cache size.
    pub cache_size: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// DNS cache space currently in use.
    pub cache_used: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum TTL retained in the DNS cache.
    pub cache_max_ttl: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout for DNS-over-HTTPS requests.
    pub doh_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether remote DNS requests are accepted.
    pub allow_remote_requests: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DNS-over-HTTPS certificates are verified.
    pub verify_doh_cert: Option<bool>,
    /// VRF name.
    pub vrf: Option<String>,
}

/// Response row from `/ip/dns/cache/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DnsCacheEntry {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this dns cache entry.
    pub name: Option<String>,
    /// DNS cache record payload.
    pub data: Option<String>,
    #[serde(rename = "type")]
    /// DNS record type stored in the cache entry.
    pub record_type: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Time-to-live remaining for the DNS cache entry.
    pub ttl: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the DNS cache entry is static.
    pub static_entry: Option<bool>,
}

/// Response row from `/ip/firewall/address-list/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FirewallAddressListEntry {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// List name.
    pub list: Option<String>,
    /// Address or prefix included in the firewall address list.
    pub address: Option<String>,
    /// Comment configured on this firewall address list entry.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Creation timestamp reported by `RouterOS`.
    pub creation_time: Option<RouterOsDateTime>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/ip/firewall/connection/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FirewallConnection {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Protocol value.
    pub protocol: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Source address or source address matcher.
    pub src_address: Option<IpEndpointAddress>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Destination address or destination address matcher.
    pub dst_address: Option<IpEndpointAddress>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Reply source address.
    pub reply_src_address: Option<IpEndpointAddress>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Reply destination address.
    pub reply_dst_address: Option<IpEndpointAddress>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout remaining for the firewall connection.
    pub timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes sent in the original connection direction.
    pub orig_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes sent in the reply connection direction.
    pub repl_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets sent in the original connection direction.
    pub orig_packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets sent in the reply connection direction.
    pub repl_packets: Option<u64>,
    /// TCP state tracked for the connection.
    pub tcp_state: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether connection tracking considers the connection assured.
    pub assured: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether connection tracking has confirmed the connection.
    pub confirmed: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether connection tracking has seen reply traffic.
    pub seen_reply: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether source NAT has been applied.
    pub srcnat: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether destination NAT has been applied.
    pub dstnat: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `FastTrack` is active for the connection.
    pub fasttrack: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether hardware offload is active for the connection.
    pub hw_offload: Option<bool>,
}

/// Common response row shape for firewall filter, NAT, mangle, and raw rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FirewallRule {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Firewall chain name.
    pub chain: Option<String>,
    /// Action configured for this row.
    pub action: Option<String>,
    /// Protocol value.
    pub protocol: Option<String>,
    /// Comment configured on this firewall rule.
    pub comment: Option<String>,
    /// Connection states matched by the firewall rule.
    pub connection_state: Option<String>,
    /// Source address or source address matcher.
    pub src_address: Option<String>,
    /// Destination address or destination address matcher.
    pub dst_address: Option<String>,
    /// Source address list matched by the firewall rule.
    pub src_address_list: Option<String>,
    /// Destination address list matched by the firewall rule.
    pub dst_address_list: Option<String>,
    /// Ingress interface list matched by the firewall rule.
    pub in_interface_list: Option<String>,
    /// Egress interface matched by the firewall rule.
    pub out_interface: Option<String>,
    /// Egress interface list matched by the firewall rule.
    pub out_interface_list: Option<String>,
    /// Destination ports matched by the firewall rule.
    pub dst_port: Option<String>,
    /// Source ports matched by the firewall rule.
    pub src_port: Option<String>,
    /// `IPsec` policy matcher used by the firewall rule.
    pub ipsec_policy: Option<String>,
    /// TCP flags matched by the firewall rule.
    pub tcp_flags: Option<String>,
    /// MSS value set by the firewall rule.
    pub new_mss: Option<String>,
    /// Routing mark set by the firewall rule.
    pub new_routing_mark: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Byte counter for this row.
    pub bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packet counter for this row.
    pub packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether matching firewall traffic is logged.
    pub log: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether firewall processing continues after this rule.
    pub passthrough: Option<bool>,
}

/// Response row from `/ip/service/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpService {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this ip service.
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub port: Option<u16>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Addresses allowed to access the IP service.
    pub address: Vec<IpPrefix>,
    /// Certificate associated with the service.
    pub certificate: Option<String>,
    /// TLS protocol version required by the service.
    pub tls_version: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
}

/// Response row from `/ip/pool/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpPool {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this ip pool.
    pub name: Option<String>,
    /// Address ranges included in the pool.
    pub ranges: Option<String>,
}

/// Response row from `/ip/vrf/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Vrf {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this vrf.
    pub name: Option<String>,
    /// Interfaces assigned to the VRF.
    pub interfaces: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is built in to `RouterOS`.
    pub builtin: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
}

/// Response row from `/ipv6/address/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ipv6Address {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv6 address assigned by this row.
    pub address: Option<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Actual interface selected by `RouterOS`.
    pub actual_interface: Option<InterfaceName>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Advertised addresses or link capabilities.
    pub advertise: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the IPv6 address was generated using EUI-64.
    pub eui_64: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the IPv6 address is link-local.
    pub link_local: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether duplicate address detection is disabled.
    pub no_dad: Option<bool>,
}

/// Response row from `/ipv6/neighbor/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ipv6Neighbor {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IPv6 address of the neighbor entry.
    pub address: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    /// MAC address associated with this row.
    pub mac_address: Option<MacAddress>,
    /// Current neighbor discovery status.
    pub status: Option<String>,
}

/// Response row from `/ipv6/route/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ipv6Route {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Destination address or destination address matcher.
    pub dst_address: Option<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Configured gateway value.
    pub gateway: Option<RouteGateway>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Resolved immediate gateway value.
    pub immediate_gw: Option<RouteGateway>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Routing table name.
    pub routing_table: Option<RoutingTableName>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Administrative distance used during route selection.
    pub distance: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Route scope value.
    pub scope: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is active.
    pub active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is inactive.
    pub inactive: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is a connected route.
    pub connect: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether equal-cost multipath is active for this route.
    pub ecmp: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the route is hardware offloaded.
    pub hw_offloaded: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether hardware offload is suppressed.
    pub suppress_hw_offload: Option<bool>,
}

/// Response row from `/ip/cloud/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpCloud {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Public address detected by `MikroTik` cloud.
    pub public_address: Option<IpAddr>,
    /// Interval between `MikroTik` cloud DDNS updates.
    pub ddns_update_interval: Option<String>,
    /// Whether `MikroTik` cloud updates the router clock.
    pub update_time: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `MikroTik` cloud DDNS is enabled.
    pub ddns_enabled: Option<bool>,
}

/// Response row from `/ip/firewall/connection/tracking/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FirewallConnectionTracking {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum connection tracking entries.
    pub max_entries: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current number of connection tracking entries.
    pub total_entries: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Generic connection tracking timeout.
    pub generic_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// ICMP connection tracking timeout.
    pub icmp_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for established TCP connections.
    pub tcp_established_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP syn-sent state.
    pub tcp_syn_sent_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP syn-received state.
    pub tcp_syn_received_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP fin-wait state.
    pub tcp_fin_wait_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP close-wait state.
    pub tcp_close_wait_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP last-ack state.
    pub tcp_last_ack_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for TCP time-wait state.
    pub tcp_time_wait_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for closed TCP connections.
    pub tcp_close_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for excessive TCP retransmits.
    pub tcp_max_retrans_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for unacknowledged TCP data.
    pub tcp_unacked_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for UDP packets.
    pub udp_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Connection tracking timeout for UDP streams.
    pub udp_stream_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv4 connection tracking is active.
    pub active_ipv4: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv6 connection tracking is active.
    pub active_ipv6: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether loose TCP connection tracking is enabled.
    pub loose_tcp_tracking: Option<bool>,
}

/// Response row from `/ip/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpSettings {
    /// Policy for accepting ICMP redirects.
    pub accept_redirects: Option<String>,
    /// Policy for accepting source-routed packets.
    pub accept_source_route: Option<String>,
    /// Reverse-path filtering mode.
    pub rp_filter: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout used for learned ARP entries.
    pub arp_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// ICMP rate limit setting.
    pub icmp_rate_limit: Option<u32>,
    /// ICMP rate mask setting.
    pub icmp_rate_mask: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes forwarded through IPv4 fast path.
    pub ipv4_fast_path_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets forwarded through IPv4 fast path.
    pub ipv4_fast_path_packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes forwarded through IPv4 `FastTrack`.
    pub ipv4_fasttrack_bytes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets forwarded through IPv4 `FastTrack`.
    pub ipv4_fasttrack_packets: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum neighbor entries allowed.
    pub max_neighbor_entries: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether fast path forwarding is allowed.
    pub allow_fast_path: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv4 forwarding is enabled.
    pub ip_forward: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv4 fast path is currently active.
    pub ipv4_fast_path_active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv4 `FastTrack` is currently active.
    pub ipv4_fasttrack_active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether secure ICMP redirects are accepted.
    pub secure_redirects: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether ICMP redirects are sent.
    pub send_redirects: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether TCP syncookies are enabled.
    pub tcp_syncookies: Option<bool>,
}

/// Response row from `/ipv6/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ipv6Settings {
    /// Policy for accepting ICMP redirects.
    pub accept_redirects: Option<String>,
    /// Policy for accepting IPv6 router advertisements.
    pub accept_router_advertisements: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum neighbor entries allowed.
    pub max_neighbor_entries: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv6 forwarding and address handling are disabled.
    pub disable_ipv6: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether IPv6 forwarding is enabled.
    pub forward: Option<bool>,
}

/// Response row from `/ip/neighbor/discovery-settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NeighborDiscoverySettings {
    /// Interface list where neighbor discovery is enabled.
    pub discover_interface_list: Option<String>,
    /// Whether LLDP MAC/PHY configuration TLVs are advertised.
    pub lldp_mac_phy_config: Option<String>,
    /// Whether LLDP maximum-frame-size TLVs are advertised.
    pub lldp_max_frame_size: Option<String>,
    /// VLAN ID advertised in LLDP-MED network policy.
    pub lldp_med_net_policy_vlan: Option<String>,
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Protocol value.
    pub protocol: Vec<DiscoveryProtocol>,
}

/// Response row from `/ip/proxy/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpProxy {
    /// Administrator contact advertised by the proxy cache.
    pub cache_administrator: Option<String>,
    /// Filesystem path used for proxy cache storage.
    pub cache_path: Option<String>,
    /// Upstream proxy used as the parent proxy.
    pub parent_proxy: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// DSCP value applied to proxy cache hits.
    pub cache_hit_dscp: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum individual object size accepted by the proxy cache.
    pub max_cache_object_size: Option<RouterOsByteSize>,
    /// Maximum proxy cache size.
    pub max_cache_size: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum client connections allowed by the proxy.
    pub max_client_connections: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum freshness lifetime used by the proxy cache.
    pub max_fresh_time: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum upstream server connections allowed by the proxy.
    pub max_server_connections: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// TCP port used to contact the parent proxy.
    pub parent_proxy_port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Source address or source address matcher.
    pub src_address: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether proxy responses must be served from cache.
    pub always_from_cache: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether proxy requests are anonymized.
    pub anonymous: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether proxy cache data is written to disk.
    pub cache_on_disk: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the proxy serializes server connections.
    pub serialize_connections: Option<bool>,
}

/// Response row from `/ip/socks/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Socks {
    /// SOCKS authentication method.
    pub auth_method: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Idle timeout for SOCKS connections.
    pub connection_idle_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum concurrent SOCKS connections.
    pub max_connections: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// `RouterOS` version associated with this entry.
    pub version: Option<u8>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/ip/ssh/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ssh {
    /// Whether SSH forwarding is enabled.
    pub forwarding_enabled: Option<String>,
    /// SSH host key algorithm.
    pub host_key_type: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// SSH host key size in bits.
    pub host_key_size: Option<u16>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether SSH allows the none cipher.
    pub allow_none_crypto: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether SSH password login remains allowed even with keys configured.
    pub always_allow_password_login: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether SSH is restricted to stronger cryptographic algorithms.
    pub strong_crypto: Option<bool>,
}

/// Response row from `/ip/traffic-flow/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct TrafficFlow {
    /// Number of traffic-flow cache entries.
    pub cache_entries: Option<String>,
    /// Interfaces assigned to the VRF.
    pub interfaces: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout for active traffic-flow records.
    pub active_flow_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout for inactive traffic-flow records.
    pub inactive_flow_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Traffic-flow packet sampling interval.
    pub sampling_interval: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Traffic-flow packet sampling space.
    pub sampling_space: Option<u32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether traffic-flow packet sampling is enabled.
    pub packet_sampling: Option<bool>,
}

/// Response row from `/ipv6/nd/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ipv6NeighborDiscovery {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// IPv6 hop limit advertised by neighbor discovery.
    pub hop_limit: Option<String>,
    /// Interface associated with this row.
    pub interface: Option<String>,
    /// Configured MTU.
    pub mtu: Option<String>,
    /// Interval between router advertisements.
    pub ra_interval: Option<String>,
    /// Router preference advertised in IPv6 router advertisements.
    pub ra_preference: Option<String>,
    /// Reachable time advertised by IPv6 neighbor discovery.
    pub reachable_time: Option<String>,
    /// Retransmit interval advertised by IPv6 neighbor discovery.
    pub retransmit_interval: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Delay before sending IPv6 router advertisements.
    pub ra_delay: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Router lifetime advertised in IPv6 router advertisements.
    pub ra_lifetime: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether DNS information is advertised through IPv6 neighbor discovery.
    pub advertise_dns: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether MAC address information is advertised through IPv6 neighbor discovery.
    pub advertise_mac_address: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether router advertisements set the managed-address flag.
    pub managed_address_configuration: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether router advertisements set the other-configuration flag.
    pub other_configuration: Option<bool>,
}

/// Response row from `/ip/pool/used/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpPoolUsed {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Address allocated from the pool.
    pub address: Option<IpAddr>,
    /// Allocation details for this used address-pool entry.
    pub info: Option<String>,
    /// User that owns the running script job.
    pub owner: Option<String>,
    /// Address pool that owns this allocation.
    pub pool: Option<String>,
}

/// Response row from `/ip/firewall/service-port/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct FirewallServicePort {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this firewall service port.
    pub name: Option<String>,
    /// Known LAN ports in this map.
    pub ports: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout used by the SIP firewall service helper.
    pub sip_timeout: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the SIP helper allows direct media negotiation.
    pub sip_direct_media: Option<bool>,
}

/// Response row from `/ip/hotspot/profile/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct HotspotProfile {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this hotspot profile.
    pub name: Option<String>,
    /// Directory containing hotspot HTML assets.
    pub html_directory: Option<String>,
    /// Hotspot login methods accepted by the profile.
    pub login_by: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Address used by the hotspot service.
    pub hotspot_address: Option<IpAddr>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Lifetime of hotspot HTTP login cookies.
    pub http_cookie_lifetime: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether hotspot queues are installed automatically.
    pub install_hotspot_queue: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether hotspot usernames are split from domain names.
    pub split_user_domain: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether RADIUS integration is enabled for this row.
    pub use_radius: Option<bool>,
}

/// Response row from `/ip/hotspot/user/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct HotspotUser {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this hotspot user.
    pub name: Option<String>,
    /// Comment configured on this hotspot user.
    pub comment: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes received by the hotspot user.
    pub bytes_in: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Bytes transmitted by the hotspot user.
    pub bytes_out: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets received by the hotspot user.
    pub packets_in: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Packets transmitted by the hotspot user.
    pub packets_out: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Uptime of this hotspot user session.
    pub uptime: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row was created dynamically by `RouterOS`.
    pub dynamic: Option<bool>,
}

/// Response row from `/ip/ipsec/profile/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpsecProfile {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this `IPsec` profile.
    pub name: Option<String>,
    /// Diffie-Hellman groups allowed by the `IPsec` profile.
    pub dh_group: Option<String>,
    /// Encryption algorithms allowed by the `IPsec` profile.
    pub enc_algorithm: Option<String>,
    /// Hash algorithm used by the `IPsec` profile.
    pub hash_algorithm: Option<String>,
    /// `IPsec` proposal checking mode.
    pub proposal_check: Option<String>,
    /// Dead peer detection interval.
    pub dpd_interval: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Dead peer detection failures allowed before declaring a peer down.
    pub dpd_maximum_failures: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Lifetime configured for this security association or proposal.
    pub lifetime: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `IPsec` NAT traversal is enabled.
    pub nat_traversal: Option<bool>,
}

/// Response row from `/ip/ipsec/proposal/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpsecProposal {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this `IPsec` proposal.
    pub name: Option<String>,
    /// Authentication algorithms allowed by the `IPsec` proposal.
    pub auth_algorithms: Option<String>,
    /// Encryption algorithms allowed by the `IPsec` proposal.
    pub enc_algorithms: Option<String>,
    /// Perfect forward secrecy group used by the `IPsec` proposal.
    pub pfs_group: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Lifetime configured for this security association or proposal.
    pub lifetime: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
}

/// Response row from `/ip/ipsec/policy/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpsecPolicy {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Source address or source address matcher.
    pub src_address: Option<IpPrefix>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Destination address or destination address matcher.
    pub dst_address: Option<IpPrefix>,
    /// `IPsec` policy group name.
    pub group: Option<String>,
    /// `IPsec` proposal selected by the policy.
    pub proposal: Option<String>,
    /// Protocol value.
    pub protocol: Option<String>,
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
    /// Whether the `IPsec` policy is a template policy.
    pub template: Option<bool>,
}

/// Response row from `/ip/ipsec/statistics/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct IpsecStatistics {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Inbound `IPsec` processing errors.
    pub in_errors: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Inbound `IPsec` packets with no matching policy.
    pub in_no_policies: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Inbound `IPsec` packets with no matching state.
    pub in_no_states: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Inbound `IPsec` packets with invalid state.
    pub in_state_invalid: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Outbound `IPsec` processing errors.
    pub out_errors: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Outbound `IPsec` packets with no matching state.
    pub out_no_states: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Outbound `IPsec` policy errors.
    pub out_policy_errors: Option<u64>,
}

/// Counter row shared by `/ip/proxy/inserts`, `/lookups`, and `/refreshes`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ProxyCounters {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Proxy requests denied by policy.
    pub denied: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Proxy requests that failed with errors.
    pub errors: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Whether this user account or counter entry has expired.
    pub expired: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Proxy requests completed successfully.
    pub successes: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Proxy URL requests processed.
    pub url_requests: Option<u64>,
}

/// Response row from `/ip/smb/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Smb {
    /// Comment configured on this smb.
    pub comment: Option<String>,
    /// SMB workgroup or domain name.
    pub domain: Option<String>,
    /// Interfaces assigned to the VRF.
    pub interfaces: Option<String>,
    /// Current SMB service status.
    pub status: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/ip/smb/shares/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct SmbShare {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this smb share.
    pub name: Option<String>,
    /// Filesystem directory exported by the SMB share.
    pub directory: Option<String>,
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
    /// Whether the entry is read-only.
    pub read_only: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether SMB clients must use encryption.
    pub require_encryption: Option<bool>,
}

/// Response row from `/ip/upnp/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Upnp {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `UPnP` may disable the external interface.
    pub allow_disable_external_interface: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `UPnP` displays its dummy firewall rule.
    pub show_dummy_rule: Option<bool>,
}

/// Response row from `/ip/nat-pmp/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NatPmp {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;
    use core::time::Duration;

    use super::DhcpLease;
    use super::IpsecProfile;
    use super::Neighbor;
    use super::Route;
    use super::ScopedIpAddress;
    use crate::Row;

    #[test]
    fn neighbor_deserializes_typed_age() {
        let mut row = Row::new();
        row.insert(".id".into(), "*1".into());
        row.insert("age".into(), "45s".into());
        row.insert("uptime".into(), "9w1d1h37m47s".into());

        let neighbor = crate::deserialize::<Neighbor>(&row).expect("neighbor row should deserialize");

        assert_eq!(
            neighbor.age.expect("age should be present").as_duration(),
            Duration::from_secs(45)
        );
        assert_eq!(
            neighbor.uptime.expect("uptime should be present").as_duration(),
            Duration::from_secs(9 * 7 * 24 * 60 * 60 + 24 * 60 * 60 + 60 * 60 + 37 * 60 + 47)
        );
    }

    #[test]
    fn dhcp_lease_deserializes_typed_durations() {
        let mut row = Row::new();
        row.insert(".id".into(), "*D5".into());
        row.insert("age".into(), "5d23h16m26s".into());
        row.insert("expires-after".into(), "3m40s".into());
        row.insert("last-seen".into(), "1m20s".into());

        let lease = crate::deserialize::<DhcpLease>(&row).expect("DHCP lease row should deserialize");

        assert_eq!(
            lease.age.expect("age should be present").as_duration(),
            Duration::from_secs(5 * 24 * 60 * 60 + 23 * 60 * 60 + 16 * 60 + 26)
        );
        assert_eq!(
            lease.expires_after.expect("expiry should be present").as_duration(),
            Duration::from_secs(3 * 60 + 40)
        );
        assert_eq!(
            lease.last_seen.expect("last seen should be present").as_duration(),
            Duration::from_secs(80)
        );
    }

    #[test]
    fn dhcp_lease_deserializes_never_durations() {
        let mut row = Row::new();
        row.insert(".id".into(), "*AD06".into());
        row.insert("expires-after".into(), "never".into());
        row.insert("last-seen".into(), "never".into());

        let lease = crate::deserialize::<DhcpLease>(&row).expect("DHCP lease row should deserialize");

        assert_eq!(
            lease.expires_after.expect("expiry should be present").to_string(),
            "never"
        );
        assert_eq!(
            lease.last_seen.expect("last seen should be present").as_duration(),
            Duration::MAX
        );
    }

    #[test]
    fn ipsec_profile_deserializes_disabled_dpd_interval() {
        let mut row = Row::new();
        row.insert(".id".into(), "*B".into());
        row.insert("name".into(), "DigiSystem - SonicWall".into());
        row.insert("dpd-interval".into(), "disable-dpd".into());
        row.insert("lifetime".into(), "8h".into());

        let profile = crate::deserialize::<IpsecProfile>(&row).expect("IPsec profile row should deserialize");

        assert_eq!(profile.dpd_interval.as_deref(), Some("disable-dpd"));
        assert_eq!(
            profile.lifetime.expect("lifetime should be present").as_duration(),
            Duration::from_secs(8 * 60 * 60)
        );
    }

    #[test]
    fn scoped_ip_address_parses_optional_scope() {
        let scoped = "10.20.10.1%vlan_10_dmz"
            .parse::<ScopedIpAddress>()
            .expect("scoped IP address should parse");

        assert_eq!(scoped.address().to_string(), "10.20.10.1");
        assert_eq!(scoped.scope(), Some("vlan_10_dmz"));
        assert!("10.20.10.1%".parse::<ScopedIpAddress>().is_err());
    }

    #[test]
    fn route_deserializes_scoped_address_interface_and_comment() {
        let mut row = Row::new();
        row.insert(".id".into(), "*80000006".into());
        row.insert("dst-address".into(), "10.20.10.0/24".into());
        row.insert("local-address".into(), "10.20.10.1%vlan_10_dmz".into());
        row.insert("vrf-interface".into(), "ether1".into());
        row.insert("comment".into(), "mikrotik-hq-dmz".into());

        let route = crate::deserialize::<Route>(&row).expect("route row should deserialize");

        let local_address = route.local_address.expect("local address should be present");
        assert_eq!(local_address.address().to_string(), "10.20.10.1");
        assert_eq!(local_address.scope(), Some("vlan_10_dmz"));
        assert_eq!(
            route.vrf_interface.expect("VRF interface should be present").as_str(),
            "ether1"
        );
        assert_eq!(route.comment.as_deref(), Some("mikrotik-hq-dmz"));
    }

    #[test]
    fn firewall_connection_deserializes_live_typed_fields() {
        let mut row = Row::new();
        row.insert(".id".into(), "*2".into());
        row.insert("protocol".into(), "udp".into());
        row.insert("src-address".into(), "10.20.20.10".into());
        row.insert("dst-address".into(), "10.20.20.1".into());
        row.insert("timeout".into(), "1s".into());
        row.insert("orig-bytes".into(), "42".into());
        row.insert("assured".into(), "false".into());

        let connection =
            crate::deserialize::<super::FirewallConnection>(&row).expect("firewall connection row should deserialize");

        assert_eq!(connection.protocol.as_deref(), Some("udp"));
        assert_eq!(connection.timeout.expect("timeout should be present").to_string(), "1s");
        assert_eq!(connection.orig_bytes, Some(42));
        assert_eq!(connection.assured, Some(false));
    }
}
