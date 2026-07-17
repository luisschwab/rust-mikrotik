//! Shared domain abstractions built from `mikrotik-types` primitives.

use core::fmt;
use core::net::IpAddr;
use core::net::Ipv4Addr;
use core::net::Ipv6Addr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;
use crate::device::TopologyNodeKey;
use crate::primitives::interface::InterfaceName;

/// One normalized IP subnet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Subnet {
    /// Normalized network address.
    pub address: IpAddr,
    /// Network prefix length.
    pub prefix_length: u8,
}

impl Subnet {
    /// Parse and normalize an interface prefix.
    #[must_use]
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        prefix.parse().ok()
    }

    /// Return whether this network contains an IP address.
    #[must_use]
    pub fn contains(self, address: IpAddr) -> bool {
        match (self.address, address) {
            (IpAddr::V4(network_address), IpAddr::V4(address)) => {
                Self::ipv4_network_address(address, self.prefix_length)
                    .is_some_and(|address| address == network_address)
            }
            (IpAddr::V6(network_address), IpAddr::V6(address)) => {
                Self::ipv6_network_address(address, self.prefix_length)
                    .is_some_and(|address| address == network_address)
            }
            (IpAddr::V4(_), IpAddr::V6(_)) | (IpAddr::V6(_), IpAddr::V4(_)) => false,
        }
    }

    /// Return whether this prefix is small enough to represent a topology link.
    #[must_use]
    pub const fn is_link_network(self) -> bool {
        match self.address {
            IpAddr::V4(_) => self.prefix_length >= 29 && self.prefix_length < 32,
            IpAddr::V6(_) => self.prefix_length >= 126 && self.prefix_length < 128,
        }
    }

    /// Parse an IP prefix into address and prefix length.
    fn parse_prefix(prefix: &str) -> Result<(IpAddr, u8), ParseError> {
        let (address, length) = prefix.split_once('/').ok_or(ParseError::IpPrefix)?;
        let address = address.parse::<IpAddr>().map_err(|_| ParseError::IpPrefix)?;
        let length = length.parse::<u8>().map_err(|_| ParseError::IpPrefix)?;
        Ok((address, length))
    }

    /// Return the IPv4 network address for a prefix length.
    fn ipv4_network_address(address: Ipv4Addr, length: u8) -> Option<Ipv4Addr> {
        if length > 32 {
            return None;
        }

        let mask = if length == 0 {
            0
        } else {
            u32::MAX << (32 - u32::from(length))
        };
        Some(Ipv4Addr::from(u32::from(address) & mask))
    }

    /// Return the IPv6 network address for a prefix length.
    fn ipv6_network_address(address: Ipv6Addr, length: u8) -> Option<Ipv6Addr> {
        if length > 128 {
            return None;
        }

        let mask = if length == 0 {
            0
        } else {
            u128::MAX << (128 - u128::from(length))
        };
        Some(Ipv6Addr::from(u128::from(address) & mask))
    }
}

impl FromStr for Subnet {
    type Err = ParseError;

    fn from_str(prefix: &str) -> Result<Self, Self::Err> {
        let (address, prefix_length) = Self::parse_prefix(prefix)?;
        let address = match address {
            IpAddr::V4(address) => {
                IpAddr::V4(Self::ipv4_network_address(address, prefix_length).ok_or(ParseError::IpPrefix)?)
            }
            IpAddr::V6(address) => {
                IpAddr::V6(Self::ipv6_network_address(address, prefix_length).ok_or(ParseError::IpPrefix)?)
            }
        };
        Ok(Self { address, prefix_length })
    }
}

impl fmt::Display for Subnet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.address, self.prefix_length)
    }
}

/// One device/interface endpoint on a subnet.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubnetEndpoint {
    /// Device node key.
    pub node: TopologyNodeKey,
    /// Interface connected to the subnet.
    pub interface: InterfaceName,
}

/// Link classes used to describe topology relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// Transit or upstream BGP link.
    Bgp,
    /// Route next-hop link.
    Route,
    /// Internal/core-side link.
    Internal,
    /// Customer access link.
    Customer,
    /// Bridge/VLAN management fallback link.
    Management,
    /// Neighbor-derived wireless/backhaul link.
    Wireless,
    /// Neighbor-discovery anchor for an otherwise unconnected collected node.
    Fallback,
    /// Link whose type could not be classified.
    Unknown,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::net::IpAddr;

    use super::*;

    #[test]
    fn subnet_normalizes_ipv4_prefix() {
        let network = "192.0.2.9/29".parse::<Subnet>().unwrap();

        assert_eq!(network.address, IpAddr::V4("192.0.2.8".parse().unwrap()));
        assert_eq!(network.prefix_length, 29);
    }

    #[test]
    fn subnet_normalizes_ipv6_prefix() {
        let network = "2001:db8::3/126".parse::<Subnet>().unwrap();

        assert_eq!(network.address, IpAddr::V6("2001:db8::".parse().unwrap()));
        assert_eq!(network.prefix_length, 126);
    }

    #[test]
    fn subnet_rejects_invalid_prefixes() {
        assert!(Subnet::from_prefix("192.0.2.1").is_none());
        assert!(Subnet::from_prefix("192.0.2.1/33").is_none());
        assert!(Subnet::from_prefix("2001:db8::1/129").is_none());
    }

    #[test]
    fn subnet_displays_normalized_cidr() {
        let network = "10.20.20.19/24".parse::<Subnet>().unwrap();

        assert_eq!(network.to_string(), "10.20.20.0/24");
    }

    #[test]
    fn subnet_from_prefix_matches_from_str() {
        assert_eq!(
            Subnet::from_prefix("10.20.20.19/24"),
            Some("10.20.20.19/24".parse::<Subnet>().unwrap())
        );
    }

    #[test]
    fn subnet_contains_matching_addresses() {
        let network = Subnet::from_prefix("192.0.2.9/29").unwrap();

        assert!(network.contains("192.0.2.14".parse().unwrap()));
        assert!(!network.contains("192.0.2.16".parse().unwrap()));
        assert!(!network.contains("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn subnet_classifies_link_network_boundaries() {
        assert!(!Subnet::from_prefix("192.0.2.0/28").unwrap().is_link_network());
        assert!(Subnet::from_prefix("192.0.2.0/29").unwrap().is_link_network());
        assert!(Subnet::from_prefix("192.0.2.0/31").unwrap().is_link_network());
        assert!(!Subnet::from_prefix("192.0.2.0/32").unwrap().is_link_network());
        assert!(!Subnet::from_prefix("2001:db8::/125").unwrap().is_link_network());
        assert!(Subnet::from_prefix("2001:db8::/126").unwrap().is_link_network());
        assert!(Subnet::from_prefix("2001:db8::/127").unwrap().is_link_network());
        assert!(!Subnet::from_prefix("2001:db8::/128").unwrap().is_link_network());
    }

    #[test]
    fn link_kind_uses_snake_case_serde() {
        assert_eq!(serde_json::to_string(&LinkKind::Bgp).unwrap(), "\"bgp\"");
        assert_eq!(serde_json::to_string(&LinkKind::Management).unwrap(), "\"management\"");
        assert_eq!(
            serde_json::from_str::<LinkKind>("\"customer\"").unwrap(),
            LinkKind::Customer
        );
    }
}
