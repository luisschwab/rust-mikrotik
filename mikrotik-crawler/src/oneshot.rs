//! One-shot recursive network graph crawler.

use core::fmt;
use core::net::IpAddr;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;

use mikrotik_common::debug_with_label;
use mikrotik_common::error_with_label;
use mikrotik_common::info_with_label;
use mikrotik_common::warn_with_label;
use mikrotik_graphviz::graph::build_graph_with_neighbor_evidence;
use mikrotik_graphviz::graph::model::NetworkGraph;
use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::device::DeviceSnapshot;
use mikrotik_types::target::DeviceTarget;
use mikrotik_types::topology::FailedNeighborCrawl;
use mikrotik_types::topology::InferredDeviceFailure;
use mikrotik_types::topology::InferredNeighborEvidence;
use serde::Deserialize;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::config::CrawlConfig;
use crate::connector::BinaryApiFactory;
use crate::connector::SnapshotClientConnector;
use crate::discovery::neighbor_log_label;
use crate::error::Error;
use crate::error::Result;
use crate::resolver::DirectTargetResolver;
use crate::resolver::TargetResolver;
use crate::snapshot::collect_target_snapshot;

/// Output from one crawl.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrawlReport {
    /// Device-centric network graph.
    pub graph: NetworkGraph,
    /// Targets that failed to collect.
    pub failed_targets: Vec<CrawlFailure>,
}

/// Failed target with human-readable error text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlFailure {
    /// Target address.
    pub address: String,
    /// Error text.
    pub error: String,
}

/// One queued crawl target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CrawlQueueItem {
    /// Target to collect.
    target: DeviceTarget,
    /// Recursive crawl depth.
    depth: usize,
    /// Zero-based connection attempt.
    attempt: usize,
}

/// Mutable state accumulated during one crawl.
#[derive(Debug, Default)]
struct CrawlState {
    /// Targets waiting to be collected.
    queue: VecDeque<CrawlQueueItem>,
    /// Target addresses that have already been queued or collected.
    seen_targets: HashSet<SocketAddr>,
    /// Neighbor evidence keyed by resolved target address.
    target_neighbor_evidence: BTreeMap<SocketAddr, InferredNeighborEvidence>,
    /// Neighbor evidence used to infer graph nodes.
    neighbor_evidence: Vec<InferredNeighborEvidence>,
    /// Successfully collected snapshots.
    snapshots: Vec<DeviceSnapshot>,
    /// Neighbor targets that could not be collected.
    failed_targets: Vec<CrawlFailure>,
    /// Failed neighbor evidence used by graph construction.
    failed_neighbors: Vec<FailedNeighborCrawl>,
}

impl CrawlState {
    /// Build initial state from seed targets.
    fn from_seeds<I>(seeds: I) -> Self
    where
        I: IntoIterator<Item = DeviceTarget>,
    {
        Self {
            queue: seeds
                .into_iter()
                .map(|seed| CrawlQueueItem {
                    target: seed,
                    depth: 0,
                    attempt: 0,
                })
                .collect(),
            ..Self::default()
        }
    }

    /// Convert accumulated state into a crawl report.
    fn into_report(self) -> CrawlReport {
        CrawlReport {
            graph: build_graph_with_neighbor_evidence(&self.snapshots, self.neighbor_evidence, self.failed_neighbors),
            failed_targets: self.failed_targets,
        }
    }
}

/// Recursive read-only network graph crawler.
#[derive(Clone)]
pub struct Crawler {
    /// Factory used to open read-only discovery clients.
    factory: Arc<dyn SnapshotClientConnector>,
    /// Resolver used to turn discovered neighbor addresses into connectable targets.
    target_resolver: Arc<dyn TargetResolver>,
    /// Runtime crawl limits.
    config: CrawlConfig,
}

impl Default for Crawler {
    fn default() -> Self {
        Self::new(Arc::new(BinaryApiFactory::default()))
    }
}

impl fmt::Debug for Crawler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Crawler")
            .field("factory", &"<dyn SnapshotClientConnector>")
            .field("target_resolver", &"<dyn TargetResolver>")
            .field("config", &self.config)
            .finish()
    }
}

impl Crawler {
    /// Build a crawler using a discovery client factory.
    #[must_use]
    pub fn new(factory: Arc<dyn SnapshotClientConnector>) -> Self {
        Self {
            factory,
            target_resolver: Arc::new(DirectTargetResolver),
            config: CrawlConfig::default(),
        }
    }

    /// Override crawler limits.
    #[must_use]
    pub fn with_config(mut self, config: CrawlConfig) -> Self {
        self.config = config;
        self
    }

    /// Override target resolution for discovered neighbor addresses.
    #[must_use]
    pub fn with_target_resolver(mut self, target_resolver: Arc<dyn TargetResolver>) -> Self {
        self.target_resolver = target_resolver;
        self
    }

    /// Crawl recursively from one seed target.
    ///
    /// # Errors
    ///
    /// Target failures are reported inside [`CrawlReport::failed_targets`].
    pub async fn crawl(&self, seed: DeviceTarget) -> Result<CrawlReport> {
        self.crawl_many([seed]).await
    }

    /// Crawl recursively from multiple seed targets.
    ///
    /// # Errors
    ///
    /// Target failures are reported inside [`CrawlReport::failed_targets`].
    pub async fn crawl_many<I>(&self, seeds: I) -> Result<CrawlReport>
    where
        I: IntoIterator<Item = DeviceTarget>,
    {
        let mut state = CrawlState::from_seeds(seeds);
        let mut in_flight = JoinSet::new();
        let max_concurrency = self.config.max_concurrency.max(1);

        loop {
            while in_flight.len() < max_concurrency && state.snapshots.len() + in_flight.len() < self.config.max_devices
            {
                let Some(item) = state.queue.pop_front() else {
                    break;
                };
                if item.attempt == 0 && !state.seen_targets.insert(item.target.address) {
                    debug_with_label!(
                        item.target.address,
                        "skipping already-seen target at depth {}",
                        item.depth
                    );
                    continue;
                }
                self.spawn_snapshot_task(&mut in_flight, item);
            }

            if in_flight.is_empty() {
                if state.snapshots.len() >= self.config.max_devices && !state.queue.is_empty() {
                    warn_with_label!(
                        "crawler",
                        "device limit reached: max_devices={}",
                        self.config.max_devices
                    );
                }
                break;
            }

            let Some(joined) = in_flight.join_next().await else {
                continue;
            };
            let (target, depth, attempt, result) = match joined {
                Ok(result) => result,
                Err(error) => {
                    return Err(Error::Io(std::io::Error::other(format!(
                        "crawler task failed: {error}"
                    ))));
                }
            };

            let Some(snapshot) = self.handle_snapshot_result(&mut state, &target, depth, attempt, result) else {
                continue;
            };

            if depth < self.config.max_depth {
                self.enqueue_discovered_neighbors(&target, depth, &snapshot, &mut state);
            } else {
                info_with_label!(
                    target.address,
                    "not following {} neighbor(s): max depth {} reached",
                    snapshot.neighbors.len(),
                    self.config.max_depth
                );
            }

            info_with_label!(
                target.address,
                "finished target depth={depth}; collected={} failed={} queue={} in_flight={}",
                state.snapshots.len() + 1,
                state.failed_targets.len(),
                state.queue.len(),
                in_flight.len()
            );
            state.snapshots.push(snapshot);
        }

        Ok(state.into_report())
    }

    /// Apply one completed snapshot task result to crawl state.
    fn handle_snapshot_result(
        &self,
        state: &mut CrawlState,
        target: &DeviceTarget,
        depth: usize,
        attempt: usize,
        result: Result<DeviceSnapshot>,
    ) -> Option<DeviceSnapshot> {
        match result {
            Ok(snapshot) => Some(snapshot),
            Err(error) if error.is_timeout_failure() && attempt < self.config.connect_retries => {
                let next_attempt = attempt + 1;
                warn_with_label!(
                    target.address,
                    "target timed out at depth {depth}; retrying attempt {}/{}",
                    next_attempt + 1,
                    self.config.connect_retries + 1
                );
                state.queue.push_back(CrawlQueueItem {
                    target: target.clone(),
                    depth,
                    attempt: next_attempt,
                });
                None
            }
            Err(error) => {
                record_failed_target(
                    target,
                    &error,
                    &state.target_neighbor_evidence,
                    &mut state.failed_neighbors,
                    &mut state.failed_targets,
                );
                None
            }
        }
    }

    /// Spawn one device snapshot job.
    fn spawn_snapshot_task(
        &self,
        in_flight: &mut JoinSet<(DeviceTarget, usize, usize, Result<DeviceSnapshot>)>,
        item: CrawlQueueItem,
    ) {
        let factory = Arc::clone(&self.factory);
        in_flight.spawn(async move {
            let target = item.target;
            info_with_label!(
                target.address,
                "collecting RouterOS discovery snapshot at depth {} attempt {}",
                item.depth,
                item.attempt + 1
            );
            let result = collect_target_snapshot(factory, &target).await;
            (target, item.depth, item.attempt, result)
        });
    }

    /// Queue recursively discovered `MikroTik` neighbors from one snapshot.
    fn enqueue_discovered_neighbors(
        &self,
        target: &DeviceTarget,
        depth: usize,
        snapshot: &DeviceSnapshot,
        state: &mut CrawlState,
    ) {
        for neighbor in &snapshot.neighbors {
            if !neighbor.is_mikrotik() {
                debug_with_label!(
                    target.address,
                    "skipping non-MikroTik neighbor {}",
                    neighbor_log_label(neighbor)
                );
                continue;
            }

            let Some(address) = neighbor.management_address() else {
                info_with_label!(
                    target.address,
                    "discovered MikroTik neighbor {} without a usable management address",
                    neighbor_log_label(neighbor)
                );
                continue;
            };
            info_with_label!(
                target.address,
                "discovered MikroTik neighbor {} address={address}",
                neighbor_log_label(neighbor)
            );

            let Some(next_target) = self.resolve_neighbor_target(target, snapshot, neighbor, address) else {
                continue;
            };

            let evidence = InferredNeighborEvidence {
                neighbor: neighbor.clone(),
                local_node: snapshot.stable_key(),
                local_interface: neighbor.interface.clone(),
            };
            state
                .target_neighbor_evidence
                .entry(next_target.address)
                .or_insert_with(|| evidence.clone());
            state.neighbor_evidence.push(evidence);

            if state.seen_targets.contains(&next_target.address) {
                debug_with_label!(
                    target.address,
                    "skipping neighbor {} address={address}: target {} already seen",
                    neighbor_log_label(neighbor),
                    next_target.address
                );
                continue;
            }

            info_with_label!(
                target.address,
                "queueing neighbor {} address={address} target={} depth={}",
                neighbor_log_label(neighbor),
                next_target.address,
                depth + 1
            );
            state.queue.push_back(CrawlQueueItem {
                target: next_target,
                depth: depth + 1,
                attempt: 0,
            });
        }
    }

    /// Resolve one discovered neighbor into the next crawl target.
    fn resolve_neighbor_target(
        &self,
        target: &DeviceTarget,
        snapshot: &DeviceSnapshot,
        neighbor: &Neighbor,
        address: IpAddr,
    ) -> Option<DeviceTarget> {
        if !self.config.address_family.includes(address) {
            debug_with_label!(
                target.address,
                "skipping neighbor {} address={address}: address family filter",
                neighbor_log_label(neighbor)
            );
            return None;
        }

        let Some(next_target) = self
            .target_resolver
            .resolve(address, &target.credentials, snapshot, neighbor)
        else {
            debug_with_label!(
                target.address,
                "skipping neighbor {} address={address}: no resolved target",
                neighbor_log_label(neighbor)
            );
            return None;
        };

        Some(next_target)
    }
}

/// Record a failed neighbor target.
fn record_failed_target(
    target: &DeviceTarget,
    error: &Error,
    neighbor_evidence: &BTreeMap<SocketAddr, InferredNeighborEvidence>,
    failed_neighbors: &mut Vec<FailedNeighborCrawl>,
    failed_targets: &mut Vec<CrawlFailure>,
) {
    if let Some(evidence) = neighbor_evidence.get(&target.address).cloned() {
        let failure = if error.is_authentication_failure() {
            InferredDeviceFailure::WrongCredentials
        } else if error.is_connection_refused() {
            InferredDeviceFailure::ApiRefused
        } else {
            InferredDeviceFailure::Unreachable
        };
        failed_neighbors.push(FailedNeighborCrawl {
            neighbor: evidence.neighbor,
            local_node: evidence.local_node,
            local_interface: evidence.local_interface,
            failure,
        });
    }
    if error.is_authentication_failure() {
        error_with_label!(target.address, "bad credentials: {error}");
    } else {
        warn_with_label!(target.address, "failed to collect device: {error}");
    }
    failed_targets.push(CrawlFailure {
        address: target.address.to_string(),
        error: error.to_string(),
    });
}
