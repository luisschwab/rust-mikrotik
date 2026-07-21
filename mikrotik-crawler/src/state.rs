//! In-memory state for the long-running crawler service.

use core::net::IpAddr;
use core::net::SocketAddr;
use core::time::Duration;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_types::device::TopologyNodeKey;
use mikrotik_types::target::DeviceTarget;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::CollectedSnapshot;
use crate::config::DEFAULT_CONNECT_TIMEOUT;
use crate::error::FailureKind;
use crate::error::Result;

/// Retry interval for rejected credentials.
const INVALID_CREDENTIALS_RETRY_INTERVAL: Duration = Duration::from_secs(30 * 60);
/// Retry interval for API-refused and unreachable targets.
const SLOW_FAILURE_RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// Retry interval for timeout and transient targets.
const FAST_FAILURE_RETRY_INTERVAL: Duration = Duration::from_secs(60);
/// Maximum timeout used for targets that repeatedly time out.
const MAX_TIMEOUT: Duration = Duration::from_secs(30);

/// In-memory state kept by the long-running crawler.
#[derive(Debug, Clone, Default)]
pub struct CrawlerStateSnapshot {
    /// Current target registry keyed by connectable target address.
    pub targets: BTreeMap<SocketAddr, DeviceTarget>,
    /// Latest successful snapshot per stable topology identity.
    pub snapshots: BTreeMap<TopologyNodeKey, CollectedSnapshot>,
    /// Latest failure text per target address.
    pub failures: BTreeMap<SocketAddr, String>,
    /// Internal retry state keyed by connectable target address.
    pub(crate) retry: BTreeMap<SocketAddr, TargetRetryState>,
}

/// Lightweight operational projection that excludes full device snapshots.
#[derive(Debug, Clone, Default)]
pub struct CrawlerStateProjection {
    /// Number of registered targets.
    pub targets: usize,
    /// Number of devices with a successful snapshot.
    pub snapshots: usize,
    /// Latest failure text per target address.
    pub failures: BTreeMap<SocketAddr, String>,
}

/// Internal retry state for one failed target.
#[derive(Debug, Clone)]
pub(crate) struct TargetRetryState {
    /// Last retry-relevant failure kind.
    kind: FailureKind,
    /// Number of consecutive failures with the same kind.
    consecutive_failures: u32,
    /// Earliest time when this target can be retried.
    next_retry_at: Instant,
    /// Connect timeout for the next attempt.
    connect_timeout: Duration,
    /// Command timeout for the next attempt.
    command_timeout: Duration,
}

/// Snapshot job selected for the current pass.
#[derive(Debug, Clone)]
pub(crate) struct SnapshotTargetJob {
    /// Target to snapshot.
    pub(crate) target: DeviceTarget,
    /// Connect timeout for this attempt.
    pub(crate) connect_timeout: Duration,
    /// Command timeout for this attempt.
    pub(crate) command_timeout: Duration,
}

/// Event emitted when crawler state changes.
#[derive(Debug, Clone)]
pub enum SnapshotEvent {
    /// A target was added to the registry.
    TargetDiscovered {
        /// Connectable target address.
        address: SocketAddr,
    },
    /// A device snapshot was refreshed.
    SnapshotUpdated {
        /// Stable topology identity.
        topology_node_key: TopologyNodeKey,
        /// Refreshed snapshot.
        snapshot: Box<CollectedSnapshot>,
    },
    /// A target collection failed.
    SnapshotFailed {
        /// Connectable target address.
        address: SocketAddr,
        /// Human-readable error.
        error: String,
    },
}

/// Publish a best-effort state event.
///
/// A broadcast send only fails when there are no active subscribers. State is
/// already committed before publication, so this condition is diagnostic and
/// must not fail collection.
pub(crate) fn publish_event(events: &broadcast::Sender<SnapshotEvent>, event: SnapshotEvent) {
    if events.send(event).is_err() {
        tracing::trace!("crawler state event had no active subscribers");
    }
}

/// Return retry-eligible snapshot targets with currently failed targets first.
pub(crate) fn snapshot_targets_by_retry_priority(
    state: &CrawlerStateSnapshot,
    now: Instant,
    default_connect_timeout: Duration,
    default_command_timeout: Duration,
) -> Vec<SnapshotTargetJob> {
    let mut targets = state
        .targets
        .values()
        .filter_map(|target| {
            let retry = state.retry.get(&target.address);
            if retry.is_some_and(|retry| retry.next_retry_at > now) {
                return None;
            }
            Some(SnapshotTargetJob {
                target: target.clone(),
                connect_timeout: retry.map_or(default_connect_timeout, |retry| retry.connect_timeout),
                command_timeout: retry.map_or(default_command_timeout, |retry| retry.command_timeout),
            })
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| {
        let left_failed = state.failures.contains_key(&left.target.address);
        let right_failed = state.failures.contains_key(&right.target.address);
        right_failed
            .cmp(&left_failed)
            .then_with(|| left.target.address.cmp(&right.target.address))
    });
    targets
}

/// Store one snapshot result in shared state.
pub(crate) async fn record_snapshot_result(
    state: &Arc<RwLock<CrawlerStateSnapshot>>,
    events: &broadcast::Sender<SnapshotEvent>,
    target: &DeviceTarget,
    result: Result<CollectedSnapshot>,
) {
    match result {
        Ok(snapshot) => {
            let topology_node_key = snapshot.topology_node_key();
            let target_aliases = snapshot_target_aliases(&snapshot);
            let event_snapshot = snapshot.clone();
            let mut state = state.write().await;
            state
                .failures
                .retain(|address, _| !target_aliases.contains(&address.ip()));
            state.retry.retain(|address, _| !target_aliases.contains(&address.ip()));
            state.snapshots.insert(topology_node_key.clone(), snapshot);
            drop(state);
            publish_event(
                events,
                SnapshotEvent::SnapshotUpdated {
                    topology_node_key,
                    snapshot: Box::new(event_snapshot),
                },
            );
        }
        Err(error) => {
            let kind = error.failure_kind();
            let error = format!("{error:#}");
            let mut state = state.write().await;
            let retry = next_retry_state(state.retry.get(&target.address), kind);
            state.failures.insert(target.address, error.clone());
            state.retry.insert(target.address, retry);
            drop(state);
            publish_event(
                events,
                SnapshotEvent::SnapshotFailed {
                    address: target.address,
                    error,
                },
            );
        }
    }
}

/// Return all address strings that should collapse onto one successful device.
fn snapshot_target_aliases(snapshot: &CollectedSnapshot) -> BTreeSet<IpAddr> {
    let mut aliases = BTreeSet::new();
    aliases.insert(snapshot.target_address.ip());
    aliases.extend(snapshot.management_addresses());
    aliases
}

/// Build retry state for a fresh failed attempt.
fn next_retry_state(previous: Option<&TargetRetryState>, kind: FailureKind) -> TargetRetryState {
    let consecutive_failures = previous.map_or(1, |previous| {
        if previous.kind == kind {
            previous.consecutive_failures.saturating_add(1)
        } else {
            1
        }
    });
    let timeout = next_timeout(previous, kind);

    TargetRetryState {
        kind,
        consecutive_failures,
        next_retry_at: Instant::now() + retry_interval(kind),
        connect_timeout: timeout,
        command_timeout: timeout,
    }
}

/// Return the interval before the next retry for one failure kind.
const fn retry_interval(kind: FailureKind) -> Duration {
    match kind {
        FailureKind::InvalidCredentials => INVALID_CREDENTIALS_RETRY_INTERVAL,
        FailureKind::ApiRefused | FailureKind::NetworkUnreachable => SLOW_FAILURE_RETRY_INTERVAL,
        FailureKind::Timeout | FailureKind::ConnectionReset | FailureKind::Other => FAST_FAILURE_RETRY_INTERVAL,
    }
}

/// Return the timeout to use on the next attempt.
fn next_timeout(previous: Option<&TargetRetryState>, kind: FailureKind) -> Duration {
    if kind == FailureKind::Timeout {
        previous
            .map_or(DEFAULT_CONNECT_TIMEOUT.saturating_mul(2), |previous| {
                previous.connect_timeout.saturating_mul(2)
            })
            .min(MAX_TIMEOUT)
    } else {
        DEFAULT_CONNECT_TIMEOUT
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use mikrotik_types::target::Credentials;
    use mikrotik_types::target::DeviceTarget;

    use super::*;
    use crate::config::DEFAULT_COMMAND_TIMEOUT;

    #[test]
    fn retry_selector_skips_targets_before_next_retry() {
        let mut state = CrawlerStateSnapshot::default();
        let address = socket("10.0.0.1");
        state.targets.insert(address, target("10.0.0.1"));
        state.failures.insert(address, "timeout".to_owned());
        state.retry.insert(
            address,
            TargetRetryState {
                kind: FailureKind::Timeout,
                consecutive_failures: 1,
                next_retry_at: Instant::now() + Duration::from_secs(60),
                connect_timeout: Duration::from_secs(10),
                command_timeout: Duration::from_secs(10),
            },
        );

        assert!(
            snapshot_targets_by_retry_priority(
                &state,
                Instant::now(),
                DEFAULT_CONNECT_TIMEOUT,
                DEFAULT_COMMAND_TIMEOUT
            )
            .is_empty()
        );
    }

    #[test]
    fn timeout_retry_doubles_timeout_until_cap() {
        let first = next_retry_state(None, FailureKind::Timeout);
        let second = next_retry_state(Some(&first), FailureKind::Timeout);
        let third = next_retry_state(Some(&second), FailureKind::Timeout);

        assert_eq!(first.connect_timeout, Duration::from_secs(10));
        assert_eq!(second.connect_timeout, Duration::from_secs(20));
        assert_eq!(third.connect_timeout, Duration::from_secs(30));
        assert_eq!(third.command_timeout, Duration::from_secs(30));
    }

    #[test]
    fn invalid_credentials_retry_uses_slow_interval() {
        let retry = next_retry_state(None, FailureKind::InvalidCredentials);
        let remaining = retry.next_retry_at.saturating_duration_since(Instant::now());

        assert!(remaining > Duration::from_secs(29 * 60));
    }

    fn target(address: &str) -> DeviceTarget {
        DeviceTarget {
            address: socket(address),
            credentials: Credentials {
                username: "admin".to_owned(),
                password: Some("password".to_owned()),
            },
        }
    }

    fn socket(address: &str) -> SocketAddr {
        format!("{address}:8728").parse().unwrap()
    }
}
