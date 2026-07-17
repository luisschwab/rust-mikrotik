//! Long-running crawler service orchestration.

use core::time::Duration;
use std::collections::btree_map::Entry;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_common::error_with_label;
use mikrotik_types::target::DeviceTarget;
use tokio::sync::Notify;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::config::CrawlerServiceConfig;
use crate::connector::RouterOsApiConnector;
use crate::connector::SnapshotClientConnector;
use crate::discovery::discovery_loop;
use crate::resolver::DirectTargetResolver;
use crate::resolver::TargetResolver;
use crate::snapshot::collect_target_snapshot_with_timeouts;
use crate::state::CrawlerStateProjection;
use crate::state::CrawlerStateSnapshot;
use crate::state::SnapshotEvent;
use crate::state::record_snapshot_result;
use crate::state::snapshot_targets_by_retry_priority;

/// Read-only handle for the long-running crawler service.
#[derive(Debug, Clone)]
pub struct CrawlerHandle {
    /// Shared crawler state.
    state: Arc<RwLock<CrawlerStateSnapshot>>,
    /// Snapshot/event broadcaster.
    events: broadcast::Sender<SnapshotEvent>,
    /// Wake-up signal for target or credential changes.
    snapshot_requested: Arc<Notify>,
}

impl CrawlerHandle {
    /// Return a consistent clone of the current crawler state.
    pub async fn state(&self) -> CrawlerStateSnapshot {
        self.state.read().await.clone()
    }

    /// Return counts and failures without cloning full device snapshots.
    pub async fn projection(&self) -> CrawlerStateProjection {
        let state = self.state.read().await;
        CrawlerStateProjection {
            targets: state.targets.len(),
            snapshots: state.snapshots.len(),
            failures: state.failures.clone(),
        }
    }

    /// Subscribe to crawler state changes.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SnapshotEvent> {
        self.events.subscribe()
    }

    /// Insert or replace a target and request an immediate collection pass.
    pub async fn upsert_target(&self, target: DeviceTarget) {
        let address = target.address;
        self.state.write().await.targets.insert(address, target);
        let _ = self.events.send(SnapshotEvent::TargetDiscovered { address });
        self.snapshot_requested.notify_one();
    }
}

/// Long-running crawler service with separate discovery and snapshot loops.
#[derive(Debug)]
pub struct CrawlerService {
    /// Read-only handle exposed to consumers.
    handle: CrawlerHandle,
    /// Background task handles aborted on drop.
    tasks: Vec<JoinHandle<()>>,
}

impl CrawlerService {
    /// Start the long-running crawler using the default binary API factory and resolver.
    #[must_use]
    pub fn start(config: &CrawlerServiceConfig) -> Self {
        let factory: Arc<dyn SnapshotClientConnector> = Arc::new(RouterOsApiConnector::new(config.protocol));
        let target_resolver: Arc<dyn TargetResolver> = Arc::new(DirectTargetResolver);
        Self::start_with_parts(config, &factory, &target_resolver)
    }

    /// Start the long-running crawler with explicit transport and target resolver dependencies.
    #[must_use]
    pub fn start_with_parts(
        config: &CrawlerServiceConfig,
        factory: &Arc<dyn SnapshotClientConnector>,
        target_resolver: &Arc<dyn TargetResolver>,
    ) -> Self {
        let (events, _) = broadcast::channel(256);
        let state = Arc::new(RwLock::new(CrawlerStateSnapshot::default()));
        let snapshot_requested = Arc::new(Notify::new());
        let handle = CrawlerHandle {
            state: Arc::clone(&state),
            events: events.clone(),
            snapshot_requested: Arc::clone(&snapshot_requested),
        };

        let snapshot_config = config.clone();
        let snapshot_task = tokio::spawn(snapshot_loop(
            Arc::clone(&state),
            events.clone(),
            Arc::clone(factory),
            Arc::clone(&snapshot_requested),
            snapshot_config,
        ));
        let discovery_task = tokio::spawn(discovery_loop(
            Arc::clone(&state),
            events,
            Arc::clone(target_resolver),
            snapshot_requested,
            config.address_family,
            config.discovery_interval,
        ));

        Self {
            handle,
            tasks: vec![snapshot_task, discovery_task],
        }
    }

    /// Return a handle for reading crawler state and subscribing to updates.
    #[must_use]
    pub fn handle(&self) -> CrawlerHandle {
        self.handle.clone()
    }
}

impl Drop for CrawlerService {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// Refresh snapshots for all registered targets on a fixed interval.
async fn snapshot_loop(
    state: Arc<RwLock<CrawlerStateSnapshot>>,
    events: broadcast::Sender<SnapshotEvent>,
    factory: Arc<dyn SnapshotClientConnector>,
    snapshot_requested: Arc<Notify>,
    config: CrawlerServiceConfig,
) {
    register_seed_targets(&state, &events, config.seeds).await;
    let mut timer = interval(config.snapshot_interval);
    timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        Box::pin(snapshot_once(
            &state,
            &events,
            Arc::clone(&factory),
            config.snapshot_concurrency,
            config.connect_timeout,
            config.command_timeout,
        ))
        .await;
        tokio::select! {
            _ = timer.tick() => {}
            () = snapshot_requested.notified() => {}
        }
    }
}

/// Register initial seed targets.
async fn register_seed_targets(
    state: &Arc<RwLock<CrawlerStateSnapshot>>,
    events: &broadcast::Sender<SnapshotEvent>,
    seeds: Vec<DeviceTarget>,
) {
    let mut state = state.write().await;
    for seed in seeds {
        let address = seed.address;
        if let Entry::Vacant(entry) = state.targets.entry(address) {
            entry.insert(seed);
            let _ = events.send(SnapshotEvent::TargetDiscovered { address });
        }
    }
}

/// Run one snapshot refresh pass for the current target registry.
async fn snapshot_once(
    state: &Arc<RwLock<CrawlerStateSnapshot>>,
    events: &broadcast::Sender<SnapshotEvent>,
    factory: Arc<dyn SnapshotClientConnector>,
    snapshot_concurrency: usize,
    connect_timeout: Duration,
    command_timeout: Duration,
) {
    let targets = {
        let state = state.read().await;
        snapshot_targets_by_retry_priority(&state, Instant::now(), connect_timeout, command_timeout)
    };
    let mut in_flight = JoinSet::new();
    let mut targets = targets.into_iter();
    let snapshot_concurrency = snapshot_concurrency.max(1);

    loop {
        while in_flight.len() < snapshot_concurrency {
            let Some(target) = targets.next() else {
                break;
            };
            let task_factory = Arc::clone(&factory);
            in_flight.spawn(async move {
                let result = collect_target_snapshot_with_timeouts(
                    task_factory,
                    &target.target,
                    target.connect_timeout,
                    target.command_timeout,
                )
                .await;
                (target.target, result)
            });
        }

        let Some(joined) = in_flight.join_next().await else {
            break;
        };
        match joined {
            Ok((target, result)) => Box::pin(record_snapshot_result(state, events, &target, result)).await,
            Err(error) => error_with_label!("crawler", "snapshot task failed: {error}"),
        }
    }
}
