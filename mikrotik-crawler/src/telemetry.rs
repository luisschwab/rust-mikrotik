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
