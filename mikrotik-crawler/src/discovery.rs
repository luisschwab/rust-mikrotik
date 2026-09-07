//! Target discovery from latest `RouterOS` neighbor snapshots.

use core::time::Duration;
use std::sync::Arc;

use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::primitives::interface::InterfaceName;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::config::AddressFamily;
use crate::resolver::TargetResolver;
use crate::state::CrawlerStateSnapshot;
use crate::state::SnapshotEvent;
use crate::state::publish_event;

/// Discover new targets from latest neighbor snapshots on a fixed interval.
pub(crate) async fn discovery_loop(
    state: Arc<RwLock<CrawlerStateSnapshot>>,
    events: broadcast::Sender<SnapshotEvent>,
    target_resolver: Arc<dyn TargetResolver>,
    snapshot_requested: Arc<Notify>,
    address_family: AddressFamily,
    discovery_interval: Duration,
) {
    let mut timer = interval(discovery_interval);
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        if discover_once(&state, &events, &target_resolver, address_family).await {
            snapshot_requested.notify_one();
        }
        timer.tick().await;
    }
}

/// Run one discovery pass from current snapshots.
async fn discover_once(
    state: &Arc<RwLock<CrawlerStateSnapshot>>,
    events: &broadcast::Sender<SnapshotEvent>,
    target_resolver: &Arc<dyn TargetResolver>,
    address_family: AddressFamily,
) -> bool {
    let state_snapshot = state.read().await.clone();
    let mut discovered = Vec::new();

    for snapshot in state_snapshot.snapshots.values() {
        let Some(source_target) = state_snapshot.targets.get(&snapshot.target_address) else {
            continue;
        };
        for neighbor in &snapshot.ip.neighbors.data {
            if !neighbor.is_mikrotik() {
                continue;
            }
            let Some(address) = neighbor.management_address() else {
                continue;
            };
            if !address_family.includes(address) {
                continue;
            }
            let Some(target) = target_resolver.resolve(address, &source_target.credentials, snapshot, neighbor) else {
                continue;
            };
            if !state_snapshot.targets.contains_key(&target.address) {
                discovered.push(target);
            }
        }
    }

    if discovered.is_empty() {
        return false;
    }

    let mut state = state.write().await;
    let mut inserted = false;
    for target in discovered {
        let address = target.address;
        if state.targets.insert(address, target).is_none() {
            inserted = true;
            publish_event(events, SnapshotEvent::TargetDiscovered { address });
        }
    }
    inserted
}

/// Return a compact log label for a neighbor row.
pub(crate) fn neighbor_log_label(neighbor: &Neighbor) -> String {
    let identity = neighbor.identity.as_deref().unwrap_or("<unknown>");
    let local_interface = neighbor.interface.as_ref().map_or("<unknown>", InterfaceName::as_str);
    let remote_interface = neighbor
        .interface_name
        .as_ref()
        .map_or("<unknown>", InterfaceName::as_str);
    format!("{identity} local_if={local_interface} remote_if={remote_interface}")
}

#[cfg(test)]
mod tests {
    use core::net::SocketAddr;

    use mikrotik_types::device::IpSnapshot;
    use mikrotik_types::device::RouterOsSnapshot;
    use mikrotik_types::target::Credentials;
    use mikrotik_types::target::DeviceTarget;
    use time::OffsetDateTime;

    use super::*;
    use crate::CollectedSnapshot;
    use crate::resolver::StaticTargetResolver;

    fn target(address: &str) -> DeviceTarget {
        DeviceTarget {
            address: address.parse().unwrap(),
            credentials: Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        }
    }

    fn snapshot(target_address: SocketAddr, neighbors: Vec<Neighbor>) -> CollectedSnapshot {
        CollectedSnapshot {
            target_address,
            collected_at: OffsetDateTime::UNIX_EPOCH,
            snapshot_duration: Duration::ZERO,
            snapshot: RouterOsSnapshot {
                ip: IpSnapshot {
                    neighbors: neighbors.into(),
                    ..IpSnapshot::default()
                },
                ..RouterOsSnapshot::default()
            },
        }
    }

    fn mikrotik_neighbor(address: &str) -> Neighbor {
        Neighbor {
            address: Some(address.parse().unwrap()),
            identity: Some("neighbor".to_owned()),
            board: Some("CHR".to_owned()),
            ..Neighbor::default()
        }
    }

    #[tokio::test]
    async fn discovery_adds_resolved_mikrotik_neighbors_once_and_publishes_event() {
        let source = target("192.0.2.1:8728");
        let mut state_value = CrawlerStateSnapshot::default();
        state_value.targets.insert(source.address, source.clone());
        state_value.snapshots.insert(
            "source".to_owned().into(),
            snapshot(source.address, vec![mikrotik_neighbor("10.0.0.2")]),
        );
        let state = Arc::new(RwLock::new(state_value));
        let (events, mut receiver) = broadcast::channel(4);
        let resolver: Arc<dyn TargetResolver> =
            Arc::new(StaticTargetResolver::new().with_target("10.0.0.2".parse().unwrap(), "127.0.0.1:18728"));

        assert!(discover_once(&state, &events, &resolver, AddressFamily::Ipv4).await);
        assert!(
            state
                .read()
                .await
                .targets
                .contains_key(&"127.0.0.1:18728".parse().unwrap())
        );
        assert!(matches!(
            receiver.recv().await.unwrap(),
            SnapshotEvent::TargetDiscovered { address } if address == "127.0.0.1:18728".parse().unwrap()
        ));
        assert!(!discover_once(&state, &events, &resolver, AddressFamily::Ipv4).await);
    }

    #[tokio::test]
    async fn discovery_filters_non_mikrotik_unspecified_wrong_family_and_unresolved_neighbors() {
        let source = target("192.0.2.1:8728");
        let neighbors = vec![
            Neighbor {
                address: Some("10.0.0.2".parse().unwrap()),
                ..Neighbor::default()
            },
            mikrotik_neighbor("0.0.0.0"),
            mikrotik_neighbor("2001:db8::2"),
            mikrotik_neighbor("10.0.0.3"),
        ];
        let mut state_value = CrawlerStateSnapshot::default();
        state_value.targets.insert(source.address, source.clone());
        state_value
            .snapshots
            .insert("source".to_owned().into(), snapshot(source.address, neighbors));
        let state = Arc::new(RwLock::new(state_value));
        let (events, _) = broadcast::channel(4);
        let resolver: Arc<dyn TargetResolver> = Arc::new(StaticTargetResolver::new());

        assert!(!discover_once(&state, &events, &resolver, AddressFamily::Ipv4).await);

        state.write().await.targets.clear();
        assert!(!discover_once(&state, &events, &resolver, AddressFamily::Any).await);
    }

    #[test]
    fn neighbor_log_labels_use_values_and_unknown_placeholders() {
        let complete = Neighbor {
            identity: Some("R02".to_owned()),
            interface: Some("ether2".parse().unwrap()),
            interface_name: Some("ether3".parse().unwrap()),
            ..Neighbor::default()
        };
        assert_eq!(neighbor_log_label(&complete), "R02 local_if=ether2 remote_if=ether3");
        assert_eq!(
            neighbor_log_label(&Neighbor::default()),
            "<unknown> local_if=<unknown> remote_if=<unknown>"
        );
    }
}
