//! Network topology models.
//!
//! These types describe discovered or operator-curated topology after endpoint
//! rows have been interpreted. They intentionally use stable identifiers and
//! optional evidence fields because LLDP, MNDP, CDP, bridge host tables, and
//! configured metadata expose different amounts of information.

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;
use core::net::SocketAddr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;
use crate::api::ip::Neighbor;
use crate::device::DeviceRole;
use crate::device::RouterOsSnapshot;
use crate::device::TopologyNodeKey;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::DiscoveryProtocol;
use crate::primitives::ip::MacAddress;
use crate::primitives::system::RouterOsVersion;

/// A discovered or operator-curated topology link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyLink {
    /// Local node identifier.
    pub local_node: TopologyNodeKey,
    /// Local interface name.
    pub local_interface: Option<InterfaceName>,
    /// Remote node identifier.
    pub remote_node: TopologyNodeKey,
    /// Remote interface name, when known.
    pub remote_interface: Option<InterfaceName>,
    /// Discovery protocols that reported this link.
    pub discovered_by: Vec<DiscoveryProtocol>,
    /// Confidence score from 0 to 100.
    pub confidence: u8,
}

impl TopologyLink {
    /// Return true when this link has BGP session or connection evidence.
    #[must_use]
    pub fn is_bgp(&self) -> bool {
        self.discovered_by.iter().any(
            |protocol| matches!(protocol, DiscoveryProtocol::Unknown(protocol) if protocol.eq_ignore_ascii_case("bgp")),
        )
    }

    /// Return true when this link is a bridge/VLAN management fallback.
    #[must_use]
    pub fn is_management(&self) -> bool {
        self.discovered_by.iter().any(
            |protocol| {
                matches!(protocol, DiscoveryProtocol::Unknown(protocol) if protocol.eq_ignore_ascii_case("management"))
            },
        )
    }

    /// Return true when this link only anchors an otherwise unconnected collected node.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        self.discovered_by.iter().any(
            |protocol| matches!(protocol, DiscoveryProtocol::Unknown(protocol) if protocol.eq_ignore_ascii_case("fallback")),
        )
    }

    /// Return true when this link has route next-hop evidence.
    #[must_use]
    pub fn is_route(&self) -> bool {
        self.discovered_by.iter().any(
            |protocol| matches!(protocol, DiscoveryProtocol::Unknown(protocol) if protocol.eq_ignore_ascii_case("route")),
        )
    }

    /// Return true when this link has neighbor-derived wireless/backhaul evidence.
    #[must_use]
    pub fn is_wireless(&self) -> bool {
        self.discovered_by.iter().any(
            |protocol| {
                matches!(protocol, DiscoveryProtocol::Unknown(protocol) if protocol.eq_ignore_ascii_case("wireless"))
            },
        )
    }
}

/// One graph node representing a `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkNode {
    /// Stable node identifier.
    pub key: TopologyNodeKey,
    /// Whether this node was successfully collected or inferred from neighbor evidence.
    pub status: NetworkNodeStatus,
    /// Inventory role used to style and arrange a collected node.
    pub role: Option<DeviceRole>,
    /// Address used to collect this node, when collected.
    pub target_address: Option<SocketAddr>,
    /// Addresses that identify this node in discovery evidence.
    pub management_addresses: Vec<IpAddr>,
    /// Collected device snapshot, when this node was reachable.
    pub snapshot: Option<RouterOsSnapshot>,
    /// Neighbor evidence used for inferred-only nodes.
    pub inferred: Option<InferredDevice>,
}

impl NetworkNode {
    /// Return a display label for graph rendering.
    #[must_use]
    pub fn label(&self) -> String {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.system.identity.name.clone())
            .or_else(|| self.inferred.as_ref().and_then(|inferred| inferred.identity.clone()))
            .unwrap_or_else(|| self.key.to_string())
    }

    /// Return a concise Graphviz label with router identity and serial number.
    #[must_use]
    pub fn graphviz_label(&self) -> String {
        let Some(snapshot) = &self.snapshot else {
            return self.label();
        };

        snapshot
            .system
            .routerboard
            .serial_number
            .as_ref()
            .map_or_else(|| self.label(), |serial| format!("{}\n{serial}", self.label()))
    }

    /// Return the collected device role.
    #[must_use]
    pub fn role(&self) -> Option<DeviceRole> {
        self.role
    }
}

/// Graph node collection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkNodeStatus {
    /// Node was collected through the `RouterOS` API.
    Collected,
    /// Node was inferred from another device's neighbor table.
    Inferred,
}

/// Device evidence inferred from another router's neighbor table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferredDevice {
    /// Neighbor management address.
    pub management_address: Option<IpAddr>,
    /// Neighbor identity or hostname.
    pub identity: Option<String>,
    /// Neighbor board, when reported.
    pub board: Option<String>,
    /// Neighbor platform, when reported.
    pub platform: Option<String>,
    /// Neighbor `RouterOS` version, when reported.
    pub version: Option<RouterOsVersion>,
    /// Neighbor MAC address, when reported.
    pub mac_address: Option<MacAddress>,
    /// Reason this inferred device could not be collected, when known.
    pub failure: Option<InferredDeviceFailure>,
}

/// Known failure reason for an inferred node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferredDeviceFailure {
    /// `RouterOS` rejected the configured credentials.
    WrongCredentials,
    /// The device is reachable but refused the `RouterOS` API TCP connection.
    #[serde(alias = "api_disabled")]
    ApiRefused,
    /// The device was discovered but could not be reached or collected.
    Unreachable,
}

/// Evidence for a crawl target that failed after being discovered from a neighbor row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedNeighborCrawl {
    /// Neighbor row that produced the failed target.
    pub neighbor: Neighbor,
    /// Collected node that discovered the failed target.
    pub local_node: TopologyNodeKey,
    /// Interface on the collected node where the failed target was discovered.
    pub local_interface: Option<InterfaceName>,
    /// Failure reason to attach to the inferred node.
    pub failure: InferredDeviceFailure,
}

/// Neighbor discovery evidence retained for a target that may fail later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredNeighborEvidence {
    /// Neighbor row that identified the target.
    pub neighbor: Neighbor,
    /// Collected node that discovered the target.
    pub local_node: TopologyNodeKey,
    /// Interface on the collected node where the target was discovered.
    pub local_interface: Option<InterfaceName>,
}

/// LAN map assembled from ARP, DHCP lease, and bridge host tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanMap {
    /// Router target used to collect the map.
    pub router: TopologyNodeKey,
    /// Known bridge ports on the router.
    pub ports: Vec<LanPort>,
    /// Hosts correlated by MAC address.
    pub hosts: Vec<LanHost>,
}

/// Bridge port shown in a LAN map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanPort {
    /// Bridge this port belongs to.
    pub bridge: Option<InterfaceName>,
    /// Bridge member interface.
    pub interface: Option<InterfaceName>,
    /// Port VLAN id.
    pub pvid: Option<u16>,
    /// Human comment from `RouterOS`.
    pub comment: Option<String>,
    /// Number of non-local bridge host rows currently seen on this port.
    pub learned_hosts: usize,
}

/// Host shown in a LAN map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanHost {
    /// Host MAC address.
    pub mac_address: MacAddress,
    /// IP addresses associated through ARP and DHCP leases.
    pub ip_addresses: Vec<IpAddr>,
    /// Hostnames reported by DHCP leases.
    pub host_names: Vec<String>,
    /// Bridge this MAC was learned on.
    pub bridge: Option<InterfaceName>,
    /// Layer-3 interface associated through ARP.
    pub layer3_interface: Option<InterfaceName>,
    /// Bridge port where this MAC was learned.
    pub bridge_port: Option<InterfaceName>,
    /// VLAN id where this MAC was learned.
    pub vlan_id: Option<u16>,
    /// `RouterOS` sources that contributed to this row.
    pub sources: Vec<LanHostSource>,
}

/// Source tables used to identify a LAN host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanHostSource {
    /// Host was seen in `/ip/arp/print`.
    Arp,
    /// Host was seen in `/ip/dhcp-server/lease/print`.
    DhcpLease,
    /// Host was seen in `/interface/bridge/host/print`.
    BridgeHost,
}

impl fmt::Display for LanHostSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Arp => "arp",
            Self::DhcpLease => "dhcp",
            Self::BridgeHost => "bridge",
        })
    }
}

impl FromStr for LanHostSource {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "arp" => Ok(Self::Arp),
            "dhcp" => Ok(Self::DhcpLease),
            "bridge" => Ok(Self::BridgeHost),
            _ => Err(ParseError::LanHostSource),
        }
    }
}
