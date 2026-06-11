//! Reusable `RouterOS` scalar and enum types.
//!
//! These types model values that appear across multiple API rows, such as row
//! IDs, interface names, MAC addresses, prefixes, durations, and stable enum
//! domains.

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

pub mod interface;
pub mod ip;
pub mod routing;
pub mod system;

pub use interface::BridgePortStatus;
pub use interface::InterfaceName;
pub use interface::InterfaceType;
pub use interface::Mtu;
pub use ip::ArpStatus;
pub use ip::DhcpLeaseStatus;
pub use ip::DiscoveryProtocol;
pub use ip::IpEndpointAddress;
pub use ip::IpPrefix;
pub use ip::MacAddress;
pub use ip::ScopedIpAddress;
pub use ip::SystemCapability;
pub use routing::BgpSessionState;
pub use routing::RouteDestination;
pub use routing::RouteGateway;
pub use routing::RoutingTableName;
pub use system::RouterOsByteSize;
pub use system::RouterOsDate;
pub use system::RouterOsDateTime;
pub use system::RouterOsDuration;
pub use system::RouterOsDurationRange;
pub use system::RouterOsTime;
pub use system::RouterOsTimeZoneOffset;
pub use system::RouterOsVersion;

/// Error returned when parsing a primitive value fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Invalid `RouterOS` row id.
    RouterOsId,
    /// Required string value was empty.
    NonEmptyString,
    /// Invalid MAC address.
    MacAddress,
    /// Invalid MTU value.
    Mtu,
    /// Invalid `RouterOS` duration.
    RouterOsDuration,
    /// Invalid `RouterOS` duration range.
    RouterOsDurationRange,
    /// Invalid `RouterOS` byte size.
    RouterOsByteSize,
    /// Invalid `RouterOS` timezone offset.
    RouterOsTimeZoneOffset,
    /// Invalid `RouterOS` date/time.
    RouterOsDateTime,
    /// Invalid `RouterOS` date.
    RouterOsDate,
    /// Invalid `RouterOS` time.
    RouterOsTime,
    /// Invalid IP prefix.
    IpPrefix,
    /// Invalid scoped IP address.
    ScopedIpAddress,
    /// Invalid IP endpoint address.
    IpEndpointAddress,
    /// Invalid device status.
    DeviceStatus,
    /// Invalid device role.
    DeviceRole,
    /// Invalid LAN host source.
    LanHostSource,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RouterOsId => formatter.write_str("invalid RouterOS row id"),
            Self::NonEmptyString => formatter.write_str("value must not be empty"),
            Self::MacAddress => formatter.write_str("invalid MAC address"),
            Self::Mtu => formatter.write_str("invalid MTU"),
            Self::RouterOsDuration => formatter.write_str("invalid RouterOS duration"),
            Self::RouterOsDurationRange => formatter.write_str("invalid RouterOS duration range"),
            Self::RouterOsByteSize => formatter.write_str("invalid RouterOS byte size"),
            Self::RouterOsTimeZoneOffset => formatter.write_str("invalid RouterOS timezone offset"),
            Self::RouterOsDateTime => formatter.write_str("invalid RouterOS date/time"),
            Self::RouterOsDate => formatter.write_str("invalid RouterOS date"),
            Self::RouterOsTime => formatter.write_str("invalid RouterOS time"),
            Self::IpPrefix => formatter.write_str("invalid IP prefix"),
            Self::ScopedIpAddress => formatter.write_str("invalid scoped IP address"),
            Self::IpEndpointAddress => formatter.write_str("invalid IP endpoint address"),
            Self::DeviceStatus => formatter.write_str("invalid device status"),
            Self::DeviceRole => formatter.write_str("invalid device role"),
            Self::LanHostSource => formatter.write_str("invalid LAN host source"),
        }
    }
}

/// Internal `RouterOS` row id, for example `*1` or `*8000000D`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RouterOsId(String);

impl RouterOsId {
    /// Return the raw `RouterOS` id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RouterOsId {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.starts_with('*') && value.len() > 1 {
            Ok(Self(value.to_owned()))
        } else {
            Err(ParseError::RouterOsId)
        }
    }
}

impl fmt::Display for RouterOsId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RouterOsId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

pub(crate) fn parse_non_empty(value: &str) -> Result<String, ParseError> {
    if value.is_empty() {
        Err(ParseError::NonEmptyString)
    } else {
        Ok(value.to_owned())
    }
}
