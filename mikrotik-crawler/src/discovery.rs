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
