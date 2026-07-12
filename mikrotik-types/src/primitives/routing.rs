//! Routing endpoint rows.
//!
//! This module models `/routing/*` responses and routing-related value types.
//! Route and nexthop rows share many wire-level fields with `/ip/route`, but
//! expose additional `RouterOS` routing-process metadata such as AFI, ownership,
//! contribution, and nexthop internals.

use alloc::borrow::ToOwned as _;
use alloc::string::String;
use alloc::string::ToString as _;
use core::convert::Infallible;
use core::fmt;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

use crate::ParseError;
use crate::parse_non_empty;
use crate::primitives::interface::InterfaceName;
use crate::primitives::ip::IpPrefix;

/// `RouterOS` routing table name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RoutingTableName(String);

impl RoutingTableName {
    /// Return the routing table name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RoutingTableName {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_non_empty(value).map(Self)
    }
}

impl fmt::Display for RoutingTableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RoutingTableName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Route gateway as `RouterOS` reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RouteGateway(String);

impl RouteGateway {
    /// Return the raw gateway string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RouteGateway {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_non_empty(value).map(Self)
    }
}

impl fmt::Display for RouteGateway {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RouteGateway {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Destination value reported by `/routing/route/print`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDestination {
    /// Destination is an IP prefix.
    Prefix(IpPrefix),
    /// Destination is an interface selector.
    Interface(InterfaceName),
}

impl FromStr for RouteDestination {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<IpPrefix>()
            .map(Self::Prefix)
            .or_else(|_| value.parse::<InterfaceName>().map(Self::Interface))
    }
}

impl fmt::Display for RouteDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prefix(prefix) => fmt::Display::fmt(prefix, f),
            Self::Interface(interface) => fmt::Display::fmt(interface, f),
        }
    }
}

impl Serialize for RouteDestination {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouteDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::RouteDestination;

    #[test]
    fn route_destination_accepts_prefixes_and_interfaces() {
        assert!(matches!(
            "0.0.0.0/0".parse::<RouteDestination>(),
            Ok(RouteDestination::Prefix(_))
        ));
        assert!(matches!(
            "ether1".parse::<RouteDestination>(),
            Ok(RouteDestination::Interface(_))
        ));
        assert!("".parse::<RouteDestination>().is_err());
    }
}

/// `RouterOS` BGP session state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BgpSessionState {
    /// Session is idle.
    Idle,
    /// Session is actively trying to connect.
    Active,
    /// TCP connection is being established.
    Connect,
    /// OPEN message was sent.
    OpenSent,
    /// OPEN message was confirmed.
    OpenConfirm,
    /// BGP session is established.
    Established,
    /// Any BGP state this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for BgpSessionState {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "idle" => Self::Idle,
            "active" => Self::Active,
            "connect" => Self::Connect,
            "open-sent" => Self::OpenSent,
            "open-confirm" => Self::OpenConfirm,
            "established" => Self::Established,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for BgpSessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Active => f.write_str("active"),
            Self::Connect => f.write_str("connect"),
            Self::OpenSent => f.write_str("open-sent"),
            Self::OpenConfirm => f.write_str("open-confirm"),
            Self::Established => f.write_str("established"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}
