//! Interface and layer-2 endpoint rows.
//!
//! This module covers rows from `/interface` and related submenus such as
//! bridges, VLANs, interface lists, Ethernet-derived settings, and `WireGuard`.
//! Common identifiers such as [`InterfaceName`] are strongly typed, while menu
//! settings that vary heavily by device model or `RouterOS` release remain
//! string-backed.

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

/// `RouterOS` interface name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct InterfaceName(String);

impl InterfaceName {
    /// Return the interface name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for InterfaceName {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_non_empty(value).map(Self)
    }
}

impl fmt::Display for InterfaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for InterfaceName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` interface kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceType {
    /// Ethernet interface.
    #[serde(rename = "ether")]
    Ethernet,
    /// Bridge interface.
    Bridge,
    /// VLAN interface.
    Vlan,
    /// Loopback interface.
    Loopback,
    /// `WireGuard` interface.
    #[serde(rename = "wg")]
    WireGuard,
    /// Any interface kind this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for InterfaceType {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "ether" => Self::Ethernet,
            "bridge" => Self::Bridge,
            "vlan" => Self::Vlan,
            "loopback" => Self::Loopback,
            "wg" => Self::WireGuard,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for InterfaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ethernet => f.write_str("ether"),
            Self::Bridge => f.write_str("bridge"),
            Self::Vlan => f.write_str("vlan"),
            Self::Loopback => f.write_str("loopback"),
            Self::WireGuard => f.write_str("wg"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

/// `RouterOS` MTU value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mtu {
    /// `RouterOS` should choose the MTU automatically.
    Auto,
    /// Explicit MTU value.
    Bytes(u32),
}

impl FromStr for Mtu {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "auto" {
            Ok(Self::Auto)
        } else {
            value.parse().map(Self::Bytes).map_err(|_| ParseError::Mtu)
        }
    }
}

impl fmt::Display for Mtu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => f.write_str("auto"),
            Self::Bytes(bytes) => write!(f, "{bytes}"),
        }
    }
}

impl Serialize for Mtu {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Mtu {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` bridge port status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BridgePortStatus {
    /// Port is active in the bridge.
    InBridge,
    /// Port is inactive.
    Inactive,
    /// Any bridge port status this version of the observer does not know yet.
    #[serde(untagged)]
    Unknown(String),
}

impl FromStr for BridgePortStatus {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "in-bridge" => Self::InBridge,
            "inactive" => Self::Inactive,
            other => Self::Unknown(other.to_owned()),
        })
    }
}

impl fmt::Display for BridgePortStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InBridge => f.write_str("in-bridge"),
            Self::Inactive => f.write_str("inactive"),
            Self::Unknown(value) => f.write_str(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn interface_names_are_non_empty_string_newtypes() {
        let name = "ether1".parse::<InterfaceName>().unwrap();
        assert_eq!(name.as_str(), "ether1");
        assert_eq!(name.to_string(), "ether1");
        assert_eq!(serde_json::to_string(&name).unwrap(), r#""ether1""#);
        assert_eq!(
            serde_json::from_str::<InterfaceName>(r#""bridge""#).unwrap().as_str(),
            "bridge"
        );
        assert_eq!("".parse::<InterfaceName>(), Err(ParseError::NonEmptyString));
        assert!(serde_json::from_str::<InterfaceName>(r#""""#).is_err());
    }

    #[test]
    fn interface_types_preserve_known_and_unknown_values() {
        for (wire, expected) in [
            ("ether", InterfaceType::Ethernet),
            ("bridge", InterfaceType::Bridge),
            ("vlan", InterfaceType::Vlan),
            ("loopback", InterfaceType::Loopback),
            ("wg", InterfaceType::WireGuard),
            ("future-type", InterfaceType::Unknown("future-type".to_string())),
        ] {
            let parsed = wire.parse::<InterfaceType>().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), wire);
        }
        assert_eq!(
            serde_json::from_str::<InterfaceType>(r#""ether""#).unwrap(),
            InterfaceType::Ethernet
        );
        assert_eq!(
            serde_json::from_str::<InterfaceType>(r#""future""#).unwrap(),
            InterfaceType::Unknown("future".to_string())
        );
    }

    #[test]
    fn mtu_parses_auto_and_numeric_values_and_uses_string_json() {
        assert_eq!("auto".parse::<Mtu>(), Ok(Mtu::Auto));
        assert_eq!("1500".parse::<Mtu>(), Ok(Mtu::Bytes(1500)));
        assert_eq!(Mtu::Auto.to_string(), "auto");
        assert_eq!(Mtu::Bytes(9000).to_string(), "9000");
        assert_eq!(serde_json::to_string(&Mtu::Bytes(1500)).unwrap(), r#""1500""#);
        assert_eq!(serde_json::from_str::<Mtu>(r#""auto""#).unwrap(), Mtu::Auto);
        assert_eq!("invalid".parse::<Mtu>(), Err(ParseError::Mtu));
        assert!(serde_json::from_str::<Mtu>(r#""invalid""#).is_err());
    }

    #[test]
    fn bridge_port_status_preserves_unknown_values() {
        for (wire, expected) in [
            ("in-bridge", BridgePortStatus::InBridge),
            ("inactive", BridgePortStatus::Inactive),
            ("future", BridgePortStatus::Unknown("future".to_string())),
        ] {
            let parsed = wire.parse::<BridgePortStatus>().unwrap();
            assert_eq!(parsed, expected);
            assert_eq!(parsed.to_string(), wire);
        }
    }
}
