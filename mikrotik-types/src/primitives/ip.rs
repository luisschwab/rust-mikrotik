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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
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
}

impl FromStr for IpPrefix {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = value.split_once('/').ok_or(ParseError::IpPrefix)?;
        let prefix = prefix.parse::<u8>().map_err(|_| ParseError::IpPrefix)?;
        let address = address.parse::<IpAddr>().map_err(|_| ParseError::IpPrefix)?;

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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permanent => formatter.write_str("permanent"),
            Self::Reachable => formatter.write_str("reachable"),
            Self::Stale => formatter.write_str("stale"),
            Self::Delay => formatter.write_str("delay"),
            Self::Unknown(value) => formatter.write_str(value),
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bound => formatter.write_str("bound"),
            Self::Waiting => formatter.write_str("waiting"),
            Self::Offered => formatter.write_str("offered"),
            Self::Busy => formatter.write_str("busy"),
            Self::Unknown(value) => formatter.write_str(value),
        }
    }
}
