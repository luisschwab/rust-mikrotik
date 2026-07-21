//! Device snapshot collection over a connected `RouterOS` API client.

mod collector;
mod sections;

use core::net::AddrParseError;
use core::net::SocketAddr;
use core::time::Duration;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_client::client::Client;
use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::interface::Interface as InterfaceCommand;
use mikrotik_client::commands::system::System;
use mikrotik_common::info_with_label;
use mikrotik_types::api::interface::Interface;
use mikrotik_types::api::system::Health;
use mikrotik_types::api::system::Resource;
use mikrotik_types::target::DeviceTarget;
use time::OffsetDateTime;

use self::collector::EndpointCollector;
use self::sections::collect_router_os_snapshot;
use crate::CollectedSnapshot;
use crate::connector::BoxFuture;
use crate::connector::DiscoveryClient;
use crate::connector::SnapshotClientConnector;
use crate::error::Error;
use crate::error::Result;
use crate::telemetry::TelemetrySnapshot;

/// Collect one target snapshot.
pub(crate) async fn collect_target_snapshot(
    connector: Arc<dyn SnapshotClientConnector>,
    target: &DeviceTarget,
) -> Result<CollectedSnapshot> {
    let client = connector.connect(target).await?;
    collect_connected_target_snapshot(client, target).await
}

/// Collect one target snapshot using per-target timeout overrides.
pub(crate) async fn collect_target_snapshot_with_timeouts(
    connector: Arc<dyn SnapshotClientConnector>,
    target: &DeviceTarget,
    connect_timeout: Duration,
    command_timeout: Duration,
) -> Result<CollectedSnapshot> {
    let client = connector
        .connect_with_timeouts(target, connect_timeout, command_timeout)
        .await?;
    collect_connected_target_snapshot(client, target).await
}

/// Collect and log a snapshot from an already connected client.
async fn collect_connected_target_snapshot(
    client: Arc<dyn DiscoveryClient>,
    target: &DeviceTarget,
) -> Result<CollectedSnapshot> {
    info_with_label!(target.address, "connected");
    let started = Instant::now();
    let target_address = target.address.to_string();
    let mut snapshot = client.snapshot(&target_address).await?;
    snapshot.snapshot_duration = started.elapsed();
    info_with_label!(
        target.address,
        "collected snapshot for device={} in {} seconds",
        snapshot.system.identity.name.as_deref().unwrap_or("<unknown>"),
        snapshot.snapshot_duration.as_secs()
    );
    Ok(snapshot)
}

/// `mikrotik-client` backed discovery client.
#[derive(Debug, Clone)]
pub(crate) struct RouterOsApiDiscoveryClient {
    /// Connected binary API client.
    pub(crate) client: Client,
    /// Maximum time spent waiting for one print command.
    pub(crate) command_timeout: Duration,
}

impl DiscoveryClient for RouterOsApiDiscoveryClient {
    fn snapshot<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<CollectedSnapshot>> {
        Box::pin(async move {
            let collector = EndpointCollector::new(target_address, &self.client, self.command_timeout);
            Ok(CollectedSnapshot {
                target_address: parse_target_address(target_address)?,
                collected_at: OffsetDateTime::now_utc(),
                snapshot_duration: Duration::ZERO,
                snapshot: Box::pin(collect_router_os_snapshot(&collector)).await?,
            })
        })
    }

    fn telemetry<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<TelemetrySnapshot>> {
        Box::pin(async move {
            let started = Instant::now();
            let collector = EndpointCollector::new(target_address, &self.client, self.command_timeout);
            let resource = collector
                .required_first::<Resource>(PrintCommand::System(System::Resource))
                .await?
                .data;
            let health = collector
                .optional_many::<Health>(PrintCommand::System(System::Health))
                .await
                .data;
            let interfaces = collector
                .optional_many::<Interface>(PrintCommand::Interface(InterfaceCommand::Interface))
                .await
                .data;

            Ok(TelemetrySnapshot {
                target_address: parse_target_address(target_address)?,
                collected_at: OffsetDateTime::now_utc(),
                collection_duration: started.elapsed(),
                resource,
                health,
                interfaces,
            })
        })
    }
}

/// Parse the API target retained with a collected result.
fn parse_target_address(target_address: &str) -> Result<SocketAddr> {
    target_address
        .parse()
        .map_err(|error: AddrParseError| Error::InvalidTarget {
            address: target_address.to_owned(),
            message: error.to_string(),
        })
}
