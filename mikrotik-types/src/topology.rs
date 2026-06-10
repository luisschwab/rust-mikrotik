//! Network topology models.
//!
//! These types describe discovered or operator-curated topology after endpoint
//! rows have been interpreted. They intentionally use stable identifiers and
//! optional evidence fields because LLDP, MNDP, CDP, bridge host tables, and
//! configured metadata expose different amounts of information.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;
use crate::device::DeviceKey;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::DiscoveryProtocol;
use crate::primitives::ip::MacAddress;

/// A discovered or operator-curated topology link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyLink {
    /// Local device key.
    pub local_device: DeviceKey,
    /// Local interface name.
    pub local_interface: Option<InterfaceName>,
    /// Remote device key.
    pub remote_device: DeviceKey,
    /// Remote interface name, when known.
    pub remote_interface: Option<InterfaceName>,
    /// Discovery protocols that reported this link.
    pub discovered_by: Vec<DiscoveryProtocol>,
    /// Confidence score from 0 to 100.
    pub confidence: u8,
}

/// LAN map assembled from ARP, DHCP lease, and bridge host tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanMap {
    /// Router target used to collect the map.
    pub router: DeviceKey,
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
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
