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
use core::net::IpAddr;
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

    /// Return the first IP next hop and optional interface scope.
    #[must_use]
    pub fn next_hop(&self) -> Option<(IpAddr, Option<InterfaceName>)> {
        let candidate = self.0.split(',').next()?.trim();
        let candidate = candidate.split_once('@').map_or(candidate, |(gateway, _table)| gateway);
        let (address, interface) = candidate
            .split_once('%')
            .map_or((candidate, None), |(address, interface)| (address, Some(interface)));
        let address = address.parse::<IpAddr>().ok()?;
        let interface = interface.and_then(|interface| interface.parse().ok());
        Some((address, interface))
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
    use alloc::format;
    use alloc::string::ToString;
    use core::net::IpAddr;

    use super::*;

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

    #[test]
    fn route_gateway_extracts_scoped_and_table_qualified_next_hops() {
        let gateway = "192.0.2.1%ether1@main,192.0.2.2".parse::<RouteGateway>().unwrap();
        let (address, interface) = gateway.next_hop().unwrap();
        assert_eq!(address, "192.0.2.1".parse::<IpAddr>().unwrap());
        assert_eq!(interface.unwrap().as_str(), "ether1");
    }

    #[test]
    fn routing_table_names_validate_and_serialize() {
        let table = "main".parse::<RoutingTableName>().unwrap();
        assert_eq!(table.as_str(), "main");
        assert_eq!(table.to_string(), "main");
        assert_eq!(serde_json::to_string(&table).unwrap(), r#""main""#);
        assert_eq!(
            serde_json::from_str::<RoutingTableName>(r#""vpn""#).unwrap().as_str(),
            "vpn"
        );
        assert!("".parse::<RoutingTableName>().is_err());
    }

    #[test]
    fn route_gateway_handles_plain_table_qualified_and_invalid_values() {
        let gateway = "192.0.2.1@main".parse::<RouteGateway>().unwrap();
        assert_eq!(gateway.as_str(), "192.0.2.1@main");
        assert_eq!(gateway.to_string(), "192.0.2.1@main");
        assert_eq!(gateway.next_hop(), Some(("192.0.2.1".parse().unwrap(), None)));
        assert_eq!(serde_json::to_string(&gateway).unwrap(), r#""192.0.2.1@main""#);
        assert_eq!(
            serde_json::from_str::<RouteGateway>(r#""2001:db8::1""#)
                .unwrap()
                .next_hop(),
            Some(("2001:db8::1".parse().unwrap(), None))
        );
        assert!("not-an-address".parse::<RouteGateway>().unwrap().next_hop().is_none());
        assert!("".parse::<RouteGateway>().is_err());
    }

    #[test]
    fn route_destinations_display_and_serialize_their_original_shape() {
        for value in ["192.0.2.0/24", "ether1"] {
            let destination = value.parse::<RouteDestination>().unwrap();
            assert_eq!(destination.to_string(), value);
            assert_eq!(serde_json::to_string(&destination).unwrap(), format!(r#""{value}""#));
            assert_eq!(
                serde_json::from_str::<RouteDestination>(&format!(r#""{value}""#)).unwrap(),
                destination
            );
        }
    }

    #[test]
    fn bgp_session_states_preserve_known_and_unknown_values() {
        for (wire, expected) in [
            ("idle", BgpSessionState::Idle),
            ("active", BgpSessionState::Active),
            ("connect", BgpSessionState::Connect),
            ("open-sent", BgpSessionState::OpenSent),
            ("open-confirm", BgpSessionState::OpenConfirm),
            ("established", BgpSessionState::Established),
            ("future", BgpSessionState::Unknown("future".to_string())),
        ] {
            let parsed = wire.parse::<BgpSessionState>().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), wire);
        }
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
