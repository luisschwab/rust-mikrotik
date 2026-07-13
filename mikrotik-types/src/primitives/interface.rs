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
