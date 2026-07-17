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
