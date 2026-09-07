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
use crate::state::publish_event;
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
        publish_event(&self.events, SnapshotEvent::TargetDiscovered { address });
        self.snapshot_requested.notify_one();
    }
}

/// Long-running crawler service with separate discovery and snapshot loops.
#[derive(Debug)]
pub struct CrawlerService {
    /// Read-only handle exposed to consumers.
    handle: CrawlerHandle,
    /// Observable background tasks.
    tasks: JoinSet<()>,
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
        let mut tasks = JoinSet::new();
        tasks.spawn(snapshot_loop(
            Arc::clone(&state),
            events.clone(),
            Arc::clone(factory),
            Arc::clone(&snapshot_requested),
            snapshot_config,
        ));
        tasks.spawn(discovery_loop(
            Arc::clone(&state),
            events,
            Arc::clone(target_resolver),
            snapshot_requested,
            config.address_family,
            config.discovery_interval,
        ));

        Self { handle, tasks }
    }

    /// Return a handle for reading crawler state and subscribing to updates.
    #[must_use]
    pub fn handle(&self) -> CrawlerHandle {
        self.handle.clone()
    }

    /// Wait until one of the crawler loops exits.
    ///
    /// A long-running crawler has no normal completion condition, so a completed
    /// loop is reported as an error for its supervisor to recover.
    ///
    /// # Errors
    ///
    /// Returns an error when a crawler loop exits or panics.
    pub async fn completed(&mut self) -> Result<(), String> {
        match self.tasks.join_next().await {
            Some(Ok(())) => Err("crawler worker exited unexpectedly".to_owned()),
            Some(Err(error)) => Err(format!("crawler worker failed: {error}")),
            None => Err("crawler has no active workers".to_owned()),
        }
    }

    /// Stop every crawler loop and wait for task cancellation to complete.
    pub async fn shutdown(&mut self) {
        self.tasks.abort_all();
        while self.tasks.join_next().await.is_some() {}
    }
}

impl Drop for CrawlerService {
    fn drop(&mut self) {
        self.tasks.abort_all();
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
            publish_event(events, SnapshotEvent::TargetDiscovered { address });
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

#[cfg(test)]
mod tests {
    use mikrotik_types::device::RouterOsSnapshot;
    use mikrotik_types::target::Credentials;
    use time::OffsetDateTime;

    use super::*;
    use crate::CollectedSnapshot;
    use crate::connector::BoxFuture;
    use crate::connector::DiscoveryClient;
    use crate::error::Error;
    use crate::error::Result;

    struct FakeClient;

    impl DiscoveryClient for FakeClient {
        fn snapshot<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<CollectedSnapshot>> {
            Box::pin(async move {
                Ok(CollectedSnapshot {
                    target_address: target_address.parse().unwrap(),
                    collected_at: OffsetDateTime::UNIX_EPOCH,
                    snapshot_duration: Duration::ZERO,
                    snapshot: RouterOsSnapshot::default(),
                })
            })
        }
    }

    struct FakeConnector {
        fail: bool,
    }

    impl SnapshotClientConnector for FakeConnector {
        fn connect<'a>(&'a self, _target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
            Box::pin(async move {
                if self.fail {
                    Err(Error::InvalidTarget {
                        address: "invalid".to_owned(),
                        message: "failed".to_owned(),
                    })
                } else {
                    Ok(Arc::new(FakeClient) as Arc<dyn DiscoveryClient>)
                }
            })
        }
    }

    fn target(address: &str, username: &str) -> DeviceTarget {
        DeviceTarget {
            address: address.parse().unwrap(),
            credentials: Credentials {
                username: username.to_owned(),
                password: None,
            },
        }
    }

    fn handle() -> CrawlerHandle {
        let (events, _) = broadcast::channel(8);
        CrawlerHandle {
            state: Arc::new(RwLock::new(CrawlerStateSnapshot::default())),
            events,
            snapshot_requested: Arc::new(Notify::new()),
        }
    }

    #[tokio::test]
    async fn handle_upserts_targets_projects_state_and_publishes_events() {
        let handle = handle();
        let mut events = handle.subscribe();
        let first = target("192.0.2.1:8728", "first");
        handle.upsert_target(first.clone()).await;
        assert!(matches!(
            events.recv().await.unwrap(),
            SnapshotEvent::TargetDiscovered { address } if address == first.address
        ));

        let replacement = target("192.0.2.1:8728", "replacement");
        handle.upsert_target(replacement.clone()).await;
        assert_eq!(
            handle
                .state()
                .await
                .targets
                .get(&replacement.address)
                .unwrap()
                .credentials
                .username,
            "replacement"
        );
        let projection = handle.projection().await;
        assert_eq!(projection.targets, 1);
        assert_eq!(projection.snapshots, 0);
        assert!(projection.failures.is_empty());
    }

    #[tokio::test]
    async fn seed_registration_deduplicates_addresses_without_replacing_credentials() {
        let state = Arc::new(RwLock::new(CrawlerStateSnapshot::default()));
        let (events, mut receiver) = broadcast::channel(8);
        register_seed_targets(
            &state,
            &events,
            vec![target("192.0.2.1:8728", "first"), target("192.0.2.1:8728", "second")],
        )
        .await;
        assert_eq!(state.read().await.targets.len(), 1);
        assert_eq!(
            state.read().await.targets.values().next().unwrap().credentials.username,
            "first"
        );
        assert!(matches!(
            receiver.recv().await.unwrap(),
            SnapshotEvent::TargetDiscovered { .. }
        ));
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn snapshot_pass_records_successes_and_failures_with_zero_concurrency_limit() {
        let state = Arc::new(RwLock::new(CrawlerStateSnapshot::default()));
        let target = target("192.0.2.1:8728", "admin");
        state.write().await.targets.insert(target.address, target.clone());
        let (events, mut receiver) = broadcast::channel(8);
        let connector: Arc<dyn SnapshotClientConnector> = Arc::new(FakeConnector { fail: false });

        snapshot_once(
            &state,
            &events,
            connector,
            0,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .await;
        assert_eq!(state.read().await.snapshots.len(), 1);
        assert!(matches!(
            receiver.recv().await.unwrap(),
            SnapshotEvent::SnapshotUpdated { .. }
        ));

        let connector: Arc<dyn SnapshotClientConnector> = Arc::new(FakeConnector { fail: true });
        snapshot_once(
            &state,
            &events,
            connector,
            1,
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .await;
        assert!(state.read().await.failures.contains_key(&target.address));
        assert!(matches!(
            receiver.recv().await.unwrap(),
            SnapshotEvent::SnapshotFailed { .. }
        ));
    }

    #[tokio::test]
    async fn completed_reports_empty_normal_and_panicking_worker_sets() {
        let mut empty = CrawlerService {
            handle: handle(),
            tasks: JoinSet::new(),
        };
        assert_eq!(empty.completed().await.unwrap_err(), "crawler has no active workers");

        let mut completed = CrawlerService {
            handle: handle(),
            tasks: JoinSet::new(),
        };
        completed.tasks.spawn(async {});
        assert_eq!(
            completed.completed().await.unwrap_err(),
            "crawler worker exited unexpectedly"
        );

        let mut panicked = CrawlerService {
            handle: handle(),
            tasks: JoinSet::new(),
        };
        panicked.tasks.spawn(async { panic!("worker panic") });
        assert!(
            panicked
                .completed()
                .await
                .unwrap_err()
                .starts_with("crawler worker failed:")
        );
    }

    #[tokio::test]
    async fn service_start_exposes_a_handle_and_shutdown_aborts_workers() {
        let config = CrawlerServiceConfig::new(vec![target("192.0.2.1:8728", "admin")]);
        let connector: Arc<dyn SnapshotClientConnector> = Arc::new(FakeConnector { fail: false });
        let resolver: Arc<dyn TargetResolver> = Arc::new(DirectTargetResolver);
        let mut service = CrawlerService::start_with_parts(&config, &connector, &resolver);
        let service_handle = service.handle();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if !service_handle.state().await.targets.is_empty() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        service.shutdown().await;
        assert!(service.tasks.is_empty());
    }
}
