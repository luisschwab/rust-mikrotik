//! Topology-specific projection of a collected device.

use core::net::IpAddr;
use core::net::SocketAddr;
use core::ops::Deref;

use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::device::TopologyNodeKey;

/// Device data required to construct a topology graph.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphSnapshot {
    /// Address used to collect the device.
    pub target_address: SocketAddr,
    /// Addresses that may identify this device in neighbor evidence.
    pub management_addresses: Vec<IpAddr>,
    /// Inventory role used for graph styling and layout.
    pub role: DeviceRole,
    /// Raw and typed `RouterOS` endpoint rows.
    pub snapshot: RouterOsSnapshot,
}

impl GraphSnapshot {
    /// Return the strongest available key, falling back to the target address.
    #[must_use]
    pub fn topology_node_key(&self) -> TopologyNodeKey {
        self.snapshot
            .topology_node_key()
            .unwrap_or_else(|| self.target_address.to_string().into())
    }
}

impl Deref for GraphSnapshot {
    type Target = RouterOsSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl From<&Self> for GraphSnapshot {
    fn from(snapshot: &Self) -> Self {
        snapshot.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use mikrotik_types::api::system::Identity;
    use mikrotik_types::device::SystemSnapshot;

    use super::*;

    fn snapshot_with_identity(identity: Option<&str>) -> GraphSnapshot {
        GraphSnapshot {
            target_address: "192.0.2.1:8728".parse().unwrap(),
            management_addresses: vec!["192.0.2.1".parse::<IpAddr>().unwrap()],
            role: DeviceRole::CoreRouter,
            snapshot: RouterOsSnapshot {
                system: SystemSnapshot {
                    identity: Identity {
                        name: identity.map(str::to_owned),
                    }
                    .into(),
                    ..SystemSnapshot::default()
                },
                ..RouterOsSnapshot::default()
            },
        }
    }

    #[test]
    fn graph_snapshot_uses_device_identity_then_target_address() {
        let named = snapshot_with_identity(Some("core"));
        assert_eq!(named.topology_node_key().as_str(), "core");
        assert!(named.system.identity.name.is_some());

        let unnamed = snapshot_with_identity(None);
        assert_eq!(unnamed.topology_node_key().as_str(), "192.0.2.1:8728");
    }

    #[test]
    fn graph_snapshot_clones_from_a_reference() {
        let snapshot = snapshot_with_identity(Some("core"));
        assert_eq!(GraphSnapshot::from(&snapshot), snapshot);
    }
}
