//! Observer-level device snapshots.
//!
//! These types are composed from lower-level `RouterOS` endpoint rows and
//! represent an observed device as a single domain object. They are intentionally
//! separate from raw endpoint structs so collection clients can keep API shape
//! changes isolated from higher-level topology and inventory logic.

use alloc::borrow::ToOwned as _;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

use crate::ParseError;
use crate::Row;
use crate::api::interface::Interface;
use crate::api::ip::Address;
use crate::api::ip::Neighbor;
use crate::api::ip::Route;
use crate::api::routing::BgpConnection;
use crate::api::routing::BgpPeer;
use crate::api::routing::BgpSession;
use crate::api::system::Identity;
use crate::api::system::Resource;
use crate::api::system::Routerboard;

/// Stable observer key for a device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceKey(String);

impl DeviceKey {
    /// Return the key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for DeviceKey {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        crate::parse_non_empty(value).map(Self)
    }
}

impl From<String> for DeviceKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Device reachability state from the observer point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    /// `RouterOS` API connection and collection succeeded.
    Reachable,
    /// The device could not be reached.
    Unreachable,
    /// `RouterOS` authentication failed.
    AuthFailed,
    /// The target is reachable but not supported by this collector.
    Unsupported,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Reachable => "reachable",
            Self::Unreachable => "unreachable",
            Self::AuthFailed => "auth_failed",
            Self::Unsupported => "unsupported",
        })
    }
}

impl FromStr for DeviceStatus {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "reachable" => Ok(Self::Reachable),
            "unreachable" => Ok(Self::Unreachable),
            "auth_failed" => Ok(Self::AuthFailed),
            "unsupported" => Ok(Self::Unsupported),
            _ => Err(ParseError::DeviceStatus),
        }
    }
}

/// Operator-assigned device role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRole {
    /// Role has not been assigned.
    Unknown,
    /// BGP edge or route server.
    BgpRouter,
    /// Core router.
    CoreRouter,
    /// Customer router.
    CustomerRouter,
    /// Switch.
    Switch,
}

impl fmt::Display for DeviceRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unknown => "unknown",
            Self::BgpRouter => "bgp_router",
            Self::CoreRouter => "core_router",
            Self::CustomerRouter => "customer_router",
            Self::Switch => "switch",
        })
    }
}

impl FromStr for DeviceRole {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "unknown" => Ok(Self::Unknown),
            "bgp_router" => Ok(Self::BgpRouter),
            "core_router" => Ok(Self::CoreRouter),
            "customer_router" => Ok(Self::CustomerRouter),
            "switch" => Ok(Self::Switch),
            _ => Err(ParseError::DeviceRole),
        }
    }
}

/// A point-in-time snapshot of one `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    /// Address used to collect this snapshot.
    pub target_address: String,
    /// Collection timestamp.
    pub collected_at: OffsetDateTime,
    /// Collection status.
    pub status: DeviceStatus,
    /// Operator-assigned role.
    pub role: DeviceRole,
    /// `/system/identity/print` row.
    pub identity: Identity,
    /// `/system/resource/print` row.
    pub resource: Resource,
    /// `/system/routerboard/print` row.
    pub routerboard: Routerboard,
    /// `/interface/print` rows.
    pub interfaces: Vec<Interface>,
    /// `/ip/neighbor/print` rows.
    pub neighbors: Vec<Neighbor>,
    /// `/ip/address/print` rows.
    pub addresses: Vec<Address>,
    /// `/ip/route/print` rows.
    pub routes: Vec<Route>,
    /// `/routing/bgp/session/print` rows.
    pub bgp_sessions: Vec<BgpSession>,
    /// `/routing/bgp/connection/print` rows.
    pub bgp_connections: Vec<BgpConnection>,
    /// `RouterOS` v6 `/routing/bgp/peer/print` rows.
    pub bgp_peers: Vec<BgpPeer>,
    /// Raw `RouterOS` rows by endpoint name.
    pub raw: BTreeMap<String, Vec<Row>>,
}

impl DeviceSnapshot {
    /// Stable-ish key used before a real inventory identity model exists.
    #[must_use]
    pub fn stable_key(&self) -> DeviceKey {
        self.routerboard
            .serial_number
            .as_deref()
            .or(self.identity.name.as_deref())
            .unwrap_or(&self.target_address)
            .to_owned()
            .into()
    }
}

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned as _;
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;

    use time::OffsetDateTime;

    use super::DeviceRole;
    use super::DeviceSnapshot;
    use super::DeviceStatus;
    use crate::api::system::Identity;
    use crate::api::system::Resource;
    use crate::api::system::Routerboard;

    #[test]
    fn snapshot_prefers_serial_for_stable_key() {
        let snapshot = DeviceSnapshot {
            target_address: "10.0.0.1".to_owned(),
            collected_at: OffsetDateTime::UNIX_EPOCH,
            status: DeviceStatus::Reachable,
            role: DeviceRole::Unknown,
            identity: Identity {
                name: Some("core".to_owned()),
            },
            resource: Resource::default(),
            routerboard: Routerboard {
                serial_number: Some("abc123".to_owned()),
                ..Routerboard::default()
            },
            interfaces: Vec::new(),
            neighbors: Vec::new(),
            addresses: Vec::new(),
            routes: Vec::new(),
            bgp_sessions: Vec::new(),
            bgp_connections: Vec::new(),
            bgp_peers: Vec::new(),
            raw: BTreeMap::new(),
        };

        assert_eq!(snapshot.stable_key().as_str(), "abc123");
    }
}
