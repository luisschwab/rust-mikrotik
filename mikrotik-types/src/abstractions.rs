//! Shared domain abstractions built from `mikrotik-types` primitives.

use core::fmt;
use core::net::IpAddr;
use core::str::FromStr;

use mikrotik_common::parse;
use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;
use crate::device::TopologyNodeKey;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::IpPrefix;

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

    /// Parse a prefix only when its address is already the canonical network address.
    #[must_use]
    pub fn from_canonical_prefix(prefix: &str) -> Option<Self> {
        let (address, prefix_length) = parse::parse_ip_prefix(prefix)?;
        let subnet = Self::from_prefix(prefix)?;
        (subnet.address == address && subnet.prefix_length == prefix_length).then_some(subnet)
    }

    /// Construct a normalized subnet from an address and prefix length.
    #[must_use]
    pub fn new(address: IpAddr, prefix_length: u8) -> Option<Self> {
        parse::network_address(address, prefix_length).map(|address| Self { address, prefix_length })
    }

    /// Return whether this network contains an IP address.
    #[must_use]
    pub fn contains(self, address: IpAddr) -> bool {
        parse::prefix_contains(self.address, self.prefix_length, address)
    }

    /// Return whether this subnet overlaps another subnet.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        parse::prefixes_overlap((self.address, self.prefix_length), (other.address, other.prefix_length))
    }

    /// Return the number of addresses in this subnet when it fits in `u128`.
    #[must_use]
    pub fn address_count(self) -> Option<u128> {
        let host_bits = parse::maximum_prefix_length(self.address) - self.prefix_length;
        (host_bits < 128).then(|| 1_u128 << host_bits)
    }

    /// Return whether this prefix is small enough to represent a topology link.
    #[must_use]
    pub const fn is_link_network(self) -> bool {
        match self.address {
            IpAddr::V4(_) => self.prefix_length >= 29 && self.prefix_length < 32,
            IpAddr::V6(_) => self.prefix_length >= 126 && self.prefix_length < 128,
        }
    }
}

impl FromStr for Subnet {
    type Err = ParseError;

    fn from_str(prefix: &str) -> Result<Self, Self::Err> {
        let (address, prefix_length) = parse::parse_ip_prefix(prefix).ok_or(ParseError::IpPrefix)?;
        let address = parse::network_address(address, prefix_length).ok_or(ParseError::IpPrefix)?;
        Ok(Self { address, prefix_length })
    }
}

impl From<&IpPrefix> for Subnet {
    fn from(prefix: &IpPrefix) -> Self {
        Self::new(prefix.address(), prefix.prefix_length())
            .expect("IpPrefix validates its address family and prefix length")
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
    fn subnet_normalizes_routeros_ip_prefix() {
        let prefix = "192.0.2.9/29".parse::<IpPrefix>().unwrap();

        assert_eq!(Subnet::from(&prefix).to_string(), "192.0.2.8/29");
    }

    #[test]
    fn subnet_contains_matching_addresses() {
        let network = Subnet::from_prefix("192.0.2.9/29").unwrap();

        assert!(network.contains("192.0.2.14".parse().unwrap()));
        assert!(!network.contains("192.0.2.16".parse().unwrap()));
        assert!(!network.contains("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn subnet_requires_canonical_prefix_when_requested() {
        assert!(Subnet::from_canonical_prefix("192.0.2.0/24").is_some());
        assert!(Subnet::from_canonical_prefix("192.0.2.1/24").is_none());
    }

    #[test]
    fn subnet_reports_overlap_and_address_count() {
        let parent = Subnet::from_prefix("192.0.2.0/24").unwrap();
        let child = Subnet::from_prefix("192.0.2.128/25").unwrap();
        let other = Subnet::from_prefix("192.0.3.0/24").unwrap();
        assert!(parent.overlaps(child));
        assert!(!parent.overlaps(other));
        assert_eq!(parent.address_count(), Some(256));
        assert_eq!(Subnet::from_prefix("::/0").unwrap().address_count(), None);
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
