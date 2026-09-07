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

#[cfg(test)]
mod tests {
    use mikrotik_types::api::ip::Address;
    use mikrotik_types::api::system::Identity;
    use mikrotik_types::api::system::Routerboard;
    use mikrotik_types::device::IpSnapshot;
    use mikrotik_types::device::SystemSnapshot;

    use super::*;

    fn collected() -> CollectedSnapshot {
        CollectedSnapshot {
            target_address: "192.0.2.1:8728".parse().unwrap(),
            collected_at: OffsetDateTime::UNIX_EPOCH,
            snapshot_duration: Duration::from_secs(1),
            snapshot: RouterOsSnapshot {
                system: SystemSnapshot {
                    identity: Identity {
                        name: Some("core".to_owned()),
                    }
                    .into(),
                    routerboard: Routerboard {
                        serial_number: Some("ABC123".to_owned()),
                        current_firmware: Some("7.21".parse().unwrap()),
                        upgrade_firmware: Some("7.22".parse().unwrap()),
                        ..Routerboard::default()
                    }
                    .into(),
                    ..SystemSnapshot::default()
                },
                ip: IpSnapshot {
                    addresses: vec![
                        Address {
                            address: Some("192.0.2.1/24".parse().unwrap()),
                            ..Address::default()
                        },
                        Address {
                            address: Some("2001:db8::1/64".parse().unwrap()),
                            ..Address::default()
                        },
                    ]
                    .into(),
                    ..IpSnapshot::default()
                },
                ..RouterOsSnapshot::default()
            },
        }
    }

    #[test]
    fn collected_snapshot_exposes_identity_addresses_and_firmware_state() {
        let collected = collected();
        assert_eq!(collected.device_serial().unwrap().as_str(), "ABC123");
        assert_eq!(collected.topology_node_key().as_str(), "ABC123");
        assert_eq!(
            collected.management_addresses(),
            ["192.0.2.1".parse::<IpAddr>().unwrap(), "2001:db8::1".parse().unwrap()]
        );
        assert!(collected.firmware_update_pending());
    }

    #[test]
    fn topology_key_falls_back_to_target_and_deref_allows_snapshot_mutation() {
        let mut collected = CollectedSnapshot {
            target_address: "192.0.2.10:8728".parse().unwrap(),
            collected_at: OffsetDateTime::UNIX_EPOCH,
            snapshot_duration: Duration::ZERO,
            snapshot: RouterOsSnapshot::default(),
        };
        assert_eq!(collected.topology_node_key().as_str(), "192.0.2.10:8728");
        collected.system.identity.data.name = Some("edge".to_owned());
        assert_eq!(collected.system.identity.name.as_deref(), Some("edge"));
        assert!(!collected.firmware_update_pending());
    }

    #[test]
    fn graph_snapshot_projection_copies_topology_inputs() {
        let collected = collected();
        let graph = GraphSnapshot::from(&collected);
        assert_eq!(graph.target_address, collected.target_address);
        assert_eq!(graph.management_addresses, collected.management_addresses());
        assert_eq!(graph.role, DeviceRole::Unknown);
        assert_eq!(graph.snapshot, collected.snapshot);
    }
}
