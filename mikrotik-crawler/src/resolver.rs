//! Target resolution for recursively discovered neighbors.
//!
//! Real networks can usually connect to the management address reported by
//! `/ip/neighbor/print`; simulated or forwarded environments often need to map
//! that discovered address to a different connectable target.

use core::net::IpAddr;
use core::net::SocketAddr;
use std::collections::BTreeMap;

use mikrotik_types::api::ip::Neighbor;
use mikrotik_types::device::DeviceSnapshot;
use mikrotik_types::target::Credentials;
use mikrotik_types::target::DeviceTarget;

/// Maps a discovered neighbor management address to the target the crawler should connect to.
pub trait TargetResolver: Send + Sync {
    /// Return the next target for a discovered neighbor.
    fn resolve(
        &self,
        address: IpAddr,
        credentials: &Credentials,
        source: &DeviceSnapshot,
        neighbor: &Neighbor,
    ) -> Option<DeviceTarget>;
}

/// Default resolver for real networks where discovered addresses are directly reachable.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirectTargetResolver;

impl TargetResolver for DirectTargetResolver {
    fn resolve(
        &self,
        address: IpAddr,
        credentials: &Credentials,
        _source: &DeviceSnapshot,
        _neighbor: &Neighbor,
    ) -> Option<DeviceTarget> {
        Some(DeviceTarget {
            address: SocketAddr::new(address, 8728),
            credentials: credentials.clone(),
        })
    }
}

/// Static address resolver for test environments such as QEMU runner scenarios.
#[derive(Debug, Clone, Default)]
pub struct StaticTargetResolver {
    /// Mapping from discovered neighbor management address to connectable target address.
    targets: BTreeMap<IpAddr, SocketAddr>,
    /// Optional credentials to use for a discovered address instead of inherited credentials.
    credentials: BTreeMap<IpAddr, Credentials>,
}

impl StaticTargetResolver {
    /// Build an empty static resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            targets: BTreeMap::new(),
            credentials: BTreeMap::new(),
        }
    }

    /// Add a mapping from a discovered address to a connectable target address.
    #[must_use]
    pub fn with_target(mut self, discovered: IpAddr, target_address: impl Into<String>) -> Self {
        if let Ok(target) = DeviceTarget::new(target_address, "", None) {
            self.targets.insert(discovered, target.address);
        }
        self
    }

    /// Override credentials for a discovered address.
    #[must_use]
    pub fn with_credentials(mut self, discovered: IpAddr, credentials: Credentials) -> Self {
        self.credentials.insert(discovered, credentials);
        self
    }
}

impl TargetResolver for StaticTargetResolver {
    fn resolve(
        &self,
        address: IpAddr,
        credentials: &Credentials,
        _source: &DeviceSnapshot,
        _neighbor: &Neighbor,
    ) -> Option<DeviceTarget> {
        self.targets.get(&address).map(|target_address| DeviceTarget {
            address: *target_address,
            credentials: self
                .credentials
                .get(&address)
                .cloned()
                .unwrap_or_else(|| credentials.clone()),
        })
    }
}
