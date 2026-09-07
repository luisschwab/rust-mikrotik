//! Lightweight high-frequency operational telemetry collection.

use core::net::SocketAddr;
use core::time::Duration;
use std::sync::Arc;

use mikrotik_client::builder::Protocol;
use mikrotik_types::api::interface::Interface;
use mikrotik_types::api::system::Health;
use mikrotik_types::api::system::Resource;
use mikrotik_types::target::DeviceTarget;
use time::OffsetDateTime;

use crate::connector::RouterOsApiConnector;
use crate::connector::SnapshotClientConnector;
use crate::error::Result;

/// Resource, health, and interface values collected without a full inventory crawl.
#[derive(Debug, Clone)]
pub struct TelemetrySnapshot {
    /// Address used for collection.
    pub target_address: SocketAddr,
    /// UTC completion timestamp.
    pub collected_at: OffsetDateTime,
    /// End-to-end connection and command duration.
    pub collection_duration: Duration,
    /// `/system/resource/print` row.
    pub resource: Resource,
    /// `/system/health/print` rows.
    pub health: Vec<Health>,
    /// `/interface/print` rows.
    pub interfaces: Vec<Interface>,
}

/// Collect one telemetry snapshot using the standard crawler transport policy.
///
/// # Errors
///
/// Returns an error when connection, authentication, or a required command fails.
pub async fn collect_target_telemetry(target: &DeviceTarget) -> Result<TelemetrySnapshot> {
    let connector: Arc<dyn SnapshotClientConnector> = Arc::new(RouterOsApiConnector::new(Protocol::Api));
    let client = connector.connect(target).await?;
    client.telemetry(&target.address.to_string()).await
}

#[cfg(test)]
mod tests {
    use mikrotik_types::target::Credentials;

    use super::*;
    use crate::snapshot::tests::MockBehavior;
    use crate::snapshot::tests::mock_router_server;

    #[tokio::test]
    async fn public_telemetry_collection_connects_and_reads_the_operational_subset() {
        let (server, address) = mock_router_server(MockBehavior::Success).await;
        let target = DeviceTarget {
            address,
            credentials: Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        };

        let telemetry = collect_target_telemetry(&target).await.unwrap();
        assert_eq!(telemetry.target_address, address);
        assert_eq!(telemetry.resource, Resource::default());
        assert!(telemetry.health.is_empty());
        assert!(telemetry.interfaces.is_empty());
        assert!(telemetry.collection_duration < Duration::from_secs(1));
        assert!(server.await.unwrap() >= 4);
    }
}
