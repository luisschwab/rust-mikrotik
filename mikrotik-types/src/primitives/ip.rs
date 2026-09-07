//! IP, IPv6, firewall, neighbor, and DHCP endpoint rows.
//!
//! This module contains address-oriented value types plus row structs for
//! `/ip/*` and selected `/ipv6/*` menus. Fields are typed when `RouterOS`
//! reports stable scalar shapes, such as addresses, prefixes, durations, MAC
//! addresses, and booleans; freer-form rule expressions and address-list values
//! intentionally stay as strings.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use alloc::string::ToString as _;
use core::convert::Infallible;
use core::fmt;
use core::net::IpAddr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;

/// MAC-48 address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    /// Return the six MAC address octets.
    #[must_use]
    pub const fn octets(self) -> [u8; 6] {
        self.0
    }
}

impl FromStr for MacAddress {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut octets = [0; 6];
        let mut parts = value.split(':');

        for octet in &mut octets {
            let part = parts.next().ok_or(ParseError::MacAddress)?;
            if part.len() != 2 {
                return Err(ParseError::MacAddress);
            }
            *octet = u8::from_str_radix(part, 16).map_err(|_| ParseError::MacAddress)?;
        }

        if parts.next().is_some() {
            return Err(ParseError::MacAddress);
        }

        Ok(Self(octets))
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl Serialize for MacAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MacAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// IP prefix in `RouterOS` slash notation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct IpPrefix(String);

impl IpPrefix {
    /// Return the prefix string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the IP address portion.
    ///
    /// # Panics
    ///
    /// Panics if the stored value is not a valid IP prefix. Values are validated
    /// when constructing or deserializing `IpPrefix`.
    #[must_use]
    pub fn address(&self) -> IpAddr {
        let address = self.0.split_once('/').map_or(self.0.as_str(), |(address, _)| address);
        address
            .split_once('%')
            .map_or(address, |(address, _)| address)
            .parse()
            .expect("IpPrefix stores only validated IP addresses")
    }

    /// Return the validated network prefix length.
    ///
    /// # Panics
    ///
    /// Panics if the stored value is not a valid IP prefix. Values are validated
    /// when constructing or deserializing `IpPrefix`.
    #[must_use]
    pub fn prefix_length(&self) -> u8 {
        self.0
            .rsplit_once('/')
            .and_then(|(_, prefix)| prefix.parse().ok())
            .expect("IpPrefix stores only validated prefix lengths")
    }

    /// Return whether this prefix identifies exactly one host address.
    #[must_use]
    pub fn is_host(&self) -> bool {
        self.prefix_length() == if self.address().is_ipv4() { 32 } else { 128 }
    }
}

impl FromStr for IpPrefix {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = value.split_once('/').ok_or(ParseError::IpPrefix)?;
        let prefix = prefix.parse::<u8>().map_err(|_| ParseError::IpPrefix)?;
        let address = address
            .split_once('%')
            .map_or(address, |(address, _)| address)
            .parse::<IpAddr>()
            .map_err(|_| ParseError::IpPrefix)?;

        let max_prefix = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };

        if prefix <= max_prefix {
            Ok(Self(value.to_owned()))
        } else {
            Err(ParseError::IpPrefix)
        }
    }
}

impl fmt::Display for IpPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IpPrefix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// IP address with an optional `RouterOS` interface scope, for example `192.168.1.31%ether1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ScopedIpAddress(String);

impl ScopedIpAddress {
    /// Return the scoped address string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the IP address portion.
    #[must_use]
    pub fn address(&self) -> IpAddr {
        let (address, _) = self.parts();
        address
    }

    /// Return the scope interface, when present.
    #[must_use]
    pub fn scope(&self) -> Option<&str> {
        self.0.split_once('%').map(|(_, scope)| scope)
    }

    /// Split the validated value into parsed IP address and optional scope.
    fn parts(&self) -> (IpAddr, Option<&str>) {
        let (address, scope) = self
            .0
            .split_once('%')
            .map_or((self.0.as_str(), None), |(address, scope)| (address, Some(scope)));

        (
            address
                .parse()
                .expect("ScopedIpAddress stores only validated IP addresses"),
            scope,
        )
    }
}

impl FromStr for ScopedIpAddress {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, scope) = value
            .split_once('%')
            .map_or((value, None), |(address, scope)| (address, Some(scope)));

        address.parse::<IpAddr>().map_err(|_| ParseError::ScopedIpAddress)?;

        if scope.is_some_and(str::is_empty) {
            return Err(ParseError::ScopedIpAddress);
        }

        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for ScopedIpAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ScopedIpAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// IP address with an optional transport port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct IpEndpointAddress(String);

impl IpEndpointAddress {
    /// Return the endpoint string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the IP address portion.
    #[must_use]
    pub fn address(&self) -> IpAddr {
        self.parts().0
    }

    /// Return the transport port, when present.
    #[must_use]
    pub fn port(&self) -> Option<u16> {
        self.parts().1
    }

    /// Split the validated value into parsed IP address and optional port.
    fn parts(&self) -> (IpAddr, Option<u16>) {
        parse_ip_endpoint_address(&self.0).expect("IpEndpointAddress stores only validated values")
    }
}

impl FromStr for IpEndpointAddress {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_ip_endpoint_address(value)?;

        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for IpEndpointAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for IpEndpointAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Parse an IP endpoint value into address and optional port components.
fn parse_ip_endpoint_address(value: &str) -> Result<(IpAddr, Option<u16>), ParseError> {
    if let Ok(address) = value.parse::<IpAddr>() {
        return Ok((address, None));
    }

    let (address, port) = value.rsplit_once(':').ok_or(ParseError::IpEndpointAddress)?;
    let address = address
        .strip_prefix('[')
        .and_then(|address| address.strip_suffix(']'))
        .unwrap_or(address);
    let address = address.parse::<IpAddr>().map_err(|_| ParseError::IpEndpointAddress)?;
    let port = port.parse::<u16>().map_err(|_| ParseError::IpEndpointAddress)?;

    Ok((address, Some(port)))
}

/// `RouterOS` discovery protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiscoveryProtocol {
    /// `MikroTik` Neighbor Discovery Protocol.
    Mndp,
    /// Link Layer Discovery Protocol.
    Lldp,
    /// Cisco Discovery Protocol.
    Cdp,
    /// Any discovery protocol this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for DiscoveryProtocol {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "mndp" => Self::Mndp,
            "lldp" => Self::Lldp,
            "cdp" => Self::Cdp,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for DiscoveryProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mndp => "mndp",
            Self::Lldp => "lldp",
            Self::Cdp => "cdp",
            Self::Unknown(protocol) => protocol.as_str(),
        })
    }
}

/// LLDP system capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SystemCapability {
    /// Bridge capability.
    Bridge,
    /// WLAN access point capability.
    WlanAp,
    /// Router capability.
    Router,
    /// Station-only capability.
    StationOnly,
    /// Any capability this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for SystemCapability {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "bridge" => Self::Bridge,
            "wlan-ap" => Self::WlanAp,
            "router" => Self::Router,
            "station-only" => Self::StationOnly,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for SystemCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bridge => "bridge",
            Self::WlanAp => "wlan-ap",
            Self::Router => "router",
            Self::StationOnly => "station-only",
            Self::Unknown(capability) => capability.as_str(),
        })
    }
}

/// `RouterOS` ARP entry status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArpStatus {
    /// Static or permanent entry.
    Permanent,
    /// Entry is reachable.
    Reachable,
    /// Entry is stale.
    Stale,
    /// Entry is in delay state.
    Delay,
    /// Any ARP status this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for ArpStatus {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "permanent" => Self::Permanent,
            "reachable" => Self::Reachable,
            "stale" => Self::Stale,
            "delay" => Self::Delay,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for ArpStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permanent => f.write_str("permanent"),
            Self::Reachable => f.write_str("reachable"),
            Self::Stale => f.write_str("stale"),
            Self::Delay => f.write_str("delay"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

/// `RouterOS` DHCP lease status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DhcpLeaseStatus {
    /// Lease is bound.
    Bound,
    /// Lease is waiting.
    Waiting,
    /// Lease was offered.
    Offered,
    /// Lease is busy.
    Busy,
    /// Any DHCP lease status this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for DhcpLeaseStatus {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "bound" => Self::Bound,
            "waiting" => Self::Waiting,
            "offered" => Self::Offered,
            "busy" => Self::Busy,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for DhcpLeaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bound => f.write_str("bound"),
            Self::Waiting => f.write_str("waiting"),
            Self::Offered => f.write_str("offered"),
            Self::Busy => f.write_str("busy"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::net::IpAddr;

    use super::*;

    #[test]
    fn mac_addresses_parse_format_and_serialize_canonically() {
        let address = "aa:bb:cc:00:01:ff".parse::<MacAddress>().unwrap();
        assert_eq!(address.octets(), [0xaa, 0xbb, 0xcc, 0, 1, 0xff]);
        assert_eq!(address.to_string(), "AA:BB:CC:00:01:FF");
        assert_eq!(serde_json::to_string(&address).unwrap(), r#""AA:BB:CC:00:01:FF""#);
        assert_eq!(
            serde_json::from_str::<MacAddress>(r#""AA:BB:CC:00:01:FF""#).unwrap(),
            address
        );

        for invalid in [
            "",
            "AA:BB",
            "AA:BB:CC:DD:EE:FF:00",
            "A:BB:CC:DD:EE:FF",
            "GG:BB:CC:DD:EE:FF",
        ] {
            assert_eq!(invalid.parse::<MacAddress>(), Err(ParseError::MacAddress));
        }
    }

    #[test]
    fn ip_prefix_accepts_scoped_ipv6_prefixes() {
        let prefix = "fe80::%ether1/64"
            .parse::<IpPrefix>()
            .expect("scoped IPv6 prefix should parse");

        assert_eq!(prefix.to_string(), "fe80::%ether1/64");
    }

    #[test]
    fn ip_prefix_rejects_invalid_scoped_ipv6_prefixes() {
        assert!("fe80::%ether1/129".parse::<IpPrefix>().is_err());
        assert!("not-an-ip%ether1/64".parse::<IpPrefix>().is_err());
    }

    #[test]
    fn ip_prefix_exposes_address_length_and_host_classification() {
        let host = "192.0.2.1/32".parse::<IpPrefix>().unwrap();
        assert_eq!(host.as_str(), "192.0.2.1/32");
        assert_eq!(host.address(), "192.0.2.1".parse::<IpAddr>().unwrap());
        assert_eq!(host.prefix_length(), 32);
        assert!(host.is_host());
        assert!(!"192.0.2.0/24".parse::<IpPrefix>().unwrap().is_host());
        assert!("2001:db8::1/128".parse::<IpPrefix>().unwrap().is_host());
        assert_eq!(serde_json::to_string(&host).unwrap(), r#""192.0.2.1/32""#);
        assert_eq!(
            serde_json::from_str::<IpPrefix>(r#""10.0.0.0/8""#).unwrap().as_str(),
            "10.0.0.0/8"
        );

        for invalid in ["192.0.2.1", "192.0.2.1/x", "192.0.2.1/33", "2001:db8::1/129"] {
            assert_eq!(invalid.parse::<IpPrefix>(), Err(ParseError::IpPrefix));
        }
    }

    #[test]
    fn scoped_addresses_expose_optional_interfaces() {
        let scoped = "fe80::1%ether1".parse::<ScopedIpAddress>().unwrap();
        assert_eq!(scoped.as_str(), "fe80::1%ether1");
        assert_eq!(scoped.address(), "fe80::1".parse::<IpAddr>().unwrap());
        assert_eq!(scoped.scope(), Some("ether1"));
        assert_eq!(scoped.to_string(), "fe80::1%ether1");
        assert_eq!(serde_json::to_string(&scoped).unwrap(), r#""fe80::1%ether1""#);

        let plain = serde_json::from_str::<ScopedIpAddress>(r#""192.0.2.1""#).unwrap();
        assert_eq!(plain.scope(), None);
        assert_eq!(plain.address(), "192.0.2.1".parse::<IpAddr>().unwrap());
        for invalid in ["not-an-ip", "192.0.2.1%"] {
            assert_eq!(invalid.parse::<ScopedIpAddress>(), Err(ParseError::ScopedIpAddress));
        }
    }

    #[test]
    fn endpoint_addresses_support_bare_and_port_qualified_ips() {
        for (wire, expected_address, expected_port) in [
            ("192.0.2.1", "192.0.2.1", None),
            ("192.0.2.1:8728", "192.0.2.1", Some(8728)),
            ("2001:db8::1", "2001:db8::1", None),
            ("[2001:db8::1]:8729", "2001:db8::1", Some(8729)),
        ] {
            let endpoint = wire.parse::<IpEndpointAddress>().unwrap();
            assert_eq!(endpoint.as_str(), wire);
            assert_eq!(endpoint.address(), expected_address.parse::<IpAddr>().unwrap());
            assert_eq!(endpoint.port(), expected_port);
            assert_eq!(endpoint.to_string(), wire);
        }
        assert_eq!(
            serde_json::from_str::<IpEndpointAddress>(r#""192.0.2.1:8728""#)
                .unwrap()
                .port(),
            Some(8728)
        );
        for invalid in ["", "host:8728", "192.0.2.1:invalid", "[2001:db8::1]:invalid"] {
            assert_eq!(invalid.parse::<IpEndpointAddress>(), Err(ParseError::IpEndpointAddress));
        }
    }

    #[test]
    fn string_backed_ip_enums_preserve_future_values() {
        for (wire, parsed) in [
            ("mndp", DiscoveryProtocol::Mndp),
            ("lldp", DiscoveryProtocol::Lldp),
            ("cdp", DiscoveryProtocol::Cdp),
            ("future", DiscoveryProtocol::Unknown("future".to_string())),
        ] {
            assert_eq!(wire.parse::<DiscoveryProtocol>().unwrap(), parsed);
            assert_eq!(parsed.to_string(), wire);
        }
        for (wire, parsed) in [
            ("bridge", SystemCapability::Bridge),
            ("wlan-ap", SystemCapability::WlanAp),
            ("router", SystemCapability::Router),
            ("station-only", SystemCapability::StationOnly),
            ("future", SystemCapability::Unknown("future".to_string())),
        ] {
            assert_eq!(wire.parse::<SystemCapability>().unwrap(), parsed);
            assert_eq!(parsed.to_string(), wire);
        }
        for (wire, parsed) in [
            ("permanent", ArpStatus::Permanent),
            ("reachable", ArpStatus::Reachable),
            ("stale", ArpStatus::Stale),
            ("delay", ArpStatus::Delay),
            ("future", ArpStatus::Unknown("future".to_string())),
        ] {
            assert_eq!(wire.parse::<ArpStatus>().unwrap(), parsed);
            assert_eq!(parsed.to_string(), wire);
        }
        for (wire, parsed) in [
            ("bound", DhcpLeaseStatus::Bound),
            ("waiting", DhcpLeaseStatus::Waiting),
            ("offered", DhcpLeaseStatus::Offered),
            ("busy", DhcpLeaseStatus::Busy),
            ("future", DhcpLeaseStatus::Unknown("future".to_string())),
        ] {
            assert_eq!(wire.parse::<DhcpLeaseStatus>().unwrap(), parsed);
            assert_eq!(parsed.to_string(), wire);
        }
    }
}
