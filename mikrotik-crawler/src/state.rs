//! In-memory state for the long-running crawler service.

use core::time::Duration;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_types::device::DeviceSnapshot;
use mikrotik_types::target::DeviceTarget;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use crate::config::DEFAULT_COMMAND_TIMEOUT;
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
    /// Latest successful snapshot per stable device serial.
    pub snapshots: BTreeMap<String, DeviceSnapshot>,
    /// Latest failure text per target address.
    pub failures: BTreeMap<SocketAddr, String>,
    /// Internal retry state keyed by connectable target address.
    pub(crate) retry: BTreeMap<SocketAddr, TargetRetryState>,
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
        /// Stable device serial.
        device_serial: String,
        /// Refreshed snapshot.
        snapshot: Box<DeviceSnapshot>,
    },
    /// A target collection failed.
    SnapshotFailed {
        /// Connectable target address.
        address: SocketAddr,
        /// Human-readable error.
        error: String,
    },
}

/// Return retry-eligible snapshot targets with currently failed targets first.
pub(crate) fn snapshot_targets_by_retry_priority(state: &CrawlerStateSnapshot, now: Instant) -> Vec<SnapshotTargetJob> {
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
                connect_timeout: retry.map_or(DEFAULT_CONNECT_TIMEOUT, |retry| retry.connect_timeout),
                command_timeout: retry.map_or(DEFAULT_COMMAND_TIMEOUT, |retry| retry.command_timeout),
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
    result: Result<DeviceSnapshot>,
) {
    match result {
        Ok(snapshot) => {
            let device_serial = snapshot.stable_key().to_string();
            let target_aliases = snapshot_target_aliases(&snapshot);
            let event_snapshot = snapshot.clone();
            let mut state = state.write().await;
            state
                .failures
                .retain(|address, _| !target_aliases.contains(&address.ip()));
            state.retry.retain(|address, _| !target_aliases.contains(&address.ip()));
            state.snapshots.insert(device_serial.clone(), snapshot);
            drop(state);
            let _ = events.send(SnapshotEvent::SnapshotUpdated {
                device_serial,
                snapshot: Box::new(event_snapshot),
            });
        }
        Err(error) => {
            let kind = error.failure_kind();
            let error = format!("{error:#}");
            let mut state = state.write().await;
            let retry = next_retry_state(state.retry.get(&target.address), kind);
            state.failures.insert(target.address, error.clone());
            state.retry.insert(target.address, retry);
            drop(state);
            let _ = events.send(SnapshotEvent::SnapshotFailed {
                address: target.address,
                error,
            });
        }
    }
}

/// Return all address strings that should collapse onto one successful device.
fn snapshot_target_aliases(snapshot: &DeviceSnapshot) -> BTreeSet<IpAddr> {
    let mut aliases = BTreeSet::new();
    aliases.insert(snapshot.target_address.ip());
    for address in &snapshot.management_addresses {
        aliases.insert(*address);
    }
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

        assert!(snapshot_targets_by_retry_priority(&state, Instant::now()).is_empty());
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
