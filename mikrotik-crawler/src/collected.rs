//! Successful collection envelope.

use core::net::IpAddr;
use core::net::SocketAddr;
use core::ops::Deref;
use core::ops::DerefMut;
use core::time::Duration;

use mikrotik_graphviz::snapshot::GraphSnapshot;
use mikrotik_types::device::DeviceRole;
use mikrotik_types::device::DeviceSerial;
use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::device::TopologyNodeKey;
use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

/// Endpoint rows collected from one target at one point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectedSnapshot {
    /// Address used to connect to the device.
    pub target_address: SocketAddr,
    /// Time at which the collection completed.
    pub collected_at: OffsetDateTime,
    /// Time spent collecting the complete `RouterOS` snapshot.
    #[serde(default)]
    pub snapshot_duration: Duration,
    /// Raw and typed `RouterOS` endpoint rows.
    pub snapshot: RouterOsSnapshot,
}

impl CollectedSnapshot {
    /// Return the `RouterBOARD` serial reported by the device.
    #[must_use]
    pub fn device_serial(&self) -> Option<DeviceSerial> {
        self.snapshot.device_serial()
    }

    /// Return the strongest topology key, falling back to the collection target.
    #[must_use]
    pub fn topology_node_key(&self) -> TopologyNodeKey {
        self.snapshot
            .topology_node_key()
            .unwrap_or_else(|| self.target_address.to_string().into())
    }

    /// Return all addresses by which this device can be identified.
    #[must_use]
    pub fn management_addresses(&self) -> Vec<IpAddr> {
        let mut addresses = vec![self.target_address.ip()];
        addresses.extend(self.snapshot.ip.addresses.data.iter().filter_map(|row| {
            row.address
                .as_ref()
                .and_then(|address| address.to_string().split('/').next()?.parse::<IpAddr>().ok())
        }));
        addresses.sort_unstable();
        addresses.dedup();
        addresses
    }

    /// Return whether the device reports a pending `RouterBOARD` firmware upgrade.
    #[must_use]
    pub fn firmware_update_pending(&self) -> bool {
        RouterOsSnapshot::routerboard_fw_update_pending(&self.snapshot.system.routerboard.data)
    }
}

impl Deref for CollectedSnapshot {
    type Target = RouterOsSnapshot;

    fn deref(&self) -> &Self::Target {
        &self.snapshot
    }
}

impl DerefMut for CollectedSnapshot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.snapshot
    }
}

impl From<&CollectedSnapshot> for GraphSnapshot {
    fn from(collected: &CollectedSnapshot) -> Self {
        Self {
            target_address: collected.target_address,
            management_addresses: collected.management_addresses(),
            role: DeviceRole::Unknown,
            snapshot: collected.snapshot.clone(),
        }
    }
}
