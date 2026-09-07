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

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use mikrotik_client::builder::ClientBuilder;
    use mikrotik_client::builder::Protocol;
    use mikrotik_client::commands::interface::Interface as InterfaceCommand;
    use mikrotik_types::api::system::Identity;
    use mikrotik_types::device::EndpointErrorKind;
    use mikrotik_types::device::RouterOsSnapshot;
    use mikrotik_types::target::Credentials;
    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;

    use super::*;

    struct FakeClient;

    impl DiscoveryClient for FakeClient {
        fn snapshot<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<CollectedSnapshot>> {
            Box::pin(async move {
                Ok(CollectedSnapshot {
                    target_address: parse_target_address(target_address)?,
                    collected_at: OffsetDateTime::UNIX_EPOCH,
                    snapshot_duration: Duration::ZERO,
                    snapshot: RouterOsSnapshot::default(),
                })
            })
        }
    }

    #[derive(Default)]
    struct FakeConnector {
        timeouts: Mutex<Option<(Duration, Duration)>>,
    }

    impl SnapshotClientConnector for FakeConnector {
        fn connect<'a>(&'a self, _target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
            Box::pin(async { Ok(Arc::new(FakeClient) as Arc<dyn DiscoveryClient>) })
        }

        fn connect_with_timeouts<'a>(
            &'a self,
            target: &'a DeviceTarget,
            connect_timeout: Duration,
            command_timeout: Duration,
        ) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
            *self.timeouts.lock().unwrap() = Some((connect_timeout, command_timeout));
            self.connect(target)
        }
    }

    fn target() -> DeviceTarget {
        DeviceTarget {
            address: "192.0.2.1:8728".parse().unwrap(),
            credentials: Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        }
    }

    pub(crate) fn encode_sentence(words: &[&[u8]]) -> Vec<u8> {
        let mut encoded = Vec::new();
        for word in words {
            assert!(word.len() < 0x80);
            encoded.push(u8::try_from(word.len()).unwrap());
            encoded.extend_from_slice(word);
        }
        encoded.push(0);
        encoded
    }

    pub(crate) async fn read_sentence(stream: &mut TcpStream, buffered: &mut Vec<u8>) -> Option<Vec<String>> {
        loop {
            let mut words = Vec::new();
            let mut offset = 0;
            while let Some(&length) = buffered.get(offset) {
                assert!(length < 0x80, "mock server only expects short command words");
                offset += 1;
                if length == 0 {
                    buffered.drain(..offset);
                    return Some(words);
                }
                let end = offset + usize::from(length);
                let Some(word) = buffered.get(offset..end) else {
                    break;
                };
                words.push(String::from_utf8(word.to_vec()).unwrap());
                offset = end;
            }

            let mut chunk = [0; 4096];
            let read = stream.read(&mut chunk).await.unwrap();
            if read == 0 {
                return None;
            }
            buffered.extend_from_slice(&chunk[..read]);
        }
    }

    #[derive(Clone, Copy)]
    pub(crate) enum MockBehavior {
        Success,
        PermissionTrap,
        Stall,
    }

    pub(crate) async fn mock_router_server(behavior: MockBehavior) -> (tokio::task::JoinHandle<usize>, SocketAddr) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffered = Vec::new();
            let mut command_count = 0;
            while let Some(words) = read_sentence(&mut stream, &mut buffered).await {
                let command = words.first().map(String::as_str).unwrap_or_default();
                let tag = words
                    .iter()
                    .find_map(|word| word.strip_prefix(".tag="))
                    .expect("every client command has a tag");
                let tag_word = format!(".tag={tag}");

                if command != "/login" && matches!(behavior, MockBehavior::PermissionTrap) {
                    stream
                        .write_all(&encode_sentence(&[
                            b"!trap",
                            tag_word.as_bytes(),
                            b"=message=not enough permissions",
                        ]))
                        .await
                        .unwrap();
                    command_count += 1;
                    continue;
                }
                if command != "/login" && matches!(behavior, MockBehavior::Stall) {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    stream
                        .write_all(&encode_sentence(&[b"!done", tag_word.as_bytes()]))
                        .await
                        .unwrap();
                    command_count += 1;
                    break;
                }
                if matches!(
                    command,
                    "/system/identity/print" | "/system/resource/print" | "/system/routerboard/print"
                ) {
                    stream
                        .write_all(&encode_sentence(&[b"!re", tag_word.as_bytes()]))
                        .await
                        .unwrap();
                }
                stream
                    .write_all(&encode_sentence(&[b"!done", tag_word.as_bytes()]))
                    .await
                    .unwrap();
                command_count += 1;
            }
            command_count
        });

        (server, address)
    }

    async fn mock_router_connection(behavior: MockBehavior) -> (Client, tokio::task::JoinHandle<usize>, SocketAddr) {
        let (server, address) = mock_router_server(behavior).await;

        let config = ClientBuilder::new(
            "127.0.0.1",
            Protocol::Api,
            Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        )
        .with_port(address.port())
        .with_connect_retry_timeout(Duration::from_secs(1));
        let client = Client::connect(config).await.unwrap();
        (client, server, address)
    }

    #[tokio::test]
    async fn snapshot_collection_connects_and_records_collection_duration() {
        let connector = Arc::new(FakeConnector::default());
        let snapshot = collect_target_snapshot(connector, &target()).await.unwrap();
        assert_eq!(snapshot.target_address, target().address);
        assert!(snapshot.snapshot_duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn snapshot_collection_forwards_timeout_overrides() {
        let connector = Arc::new(FakeConnector::default());
        let snapshot = collect_target_snapshot_with_timeouts(
            connector.clone(),
            &target(),
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .await
        .unwrap();
        assert_eq!(snapshot.target_address, target().address);
        assert_eq!(
            *connector.timeouts.lock().unwrap(),
            Some((Duration::from_secs(2), Duration::from_secs(3)))
        );
    }

    #[tokio::test]
    async fn default_telemetry_projection_uses_the_full_snapshot() {
        let telemetry = FakeClient.telemetry("192.0.2.1:8728").await.unwrap();
        assert_eq!(telemetry.target_address, target().address);
        assert_eq!(telemetry.collected_at, OffsetDateTime::UNIX_EPOCH);
        assert!(telemetry.health.is_empty());
        assert!(telemetry.interfaces.is_empty());
    }

    #[tokio::test]
    async fn routeros_client_collects_every_snapshot_section_and_telemetry_over_the_wire() {
        let (client, server, address) = mock_router_connection(MockBehavior::Success).await;
        let client = RouterOsApiDiscoveryClient {
            client,
            command_timeout: Duration::from_secs(1),
        };

        let snapshot = client.snapshot(&address.to_string()).await.unwrap();
        assert_eq!(snapshot.target_address, address);
        assert_eq!(snapshot.system.identity.data, Identity::default());
        assert!(snapshot.interface.interfaces.data.is_empty());
        assert!(snapshot.ip.neighbors.data.is_empty());
        assert!(snapshot.ipv6.ipv6_addresses.data.is_empty());
        assert!(snapshot.routing.bgp_sessions.data.is_empty());
        assert!(snapshot.queue.queue_interfaces.data.is_empty());
        assert!(snapshot.user.users.data.is_empty());

        let telemetry = client.telemetry(&address.to_string()).await.unwrap();
        assert_eq!(telemetry.target_address, address);
        assert_eq!(telemetry.resource, Resource::default());
        assert!(telemetry.health.is_empty());
        assert!(telemetry.interfaces.is_empty());

        drop(client);
        let command_count = tokio::time::timeout(Duration::from_secs(1), server)
            .await
            .unwrap()
            .unwrap();
        assert!(command_count > 100, "expected all snapshot endpoints to be queried");
    }

    #[tokio::test]
    async fn endpoint_collector_preserves_required_optional_and_empty_failures() {
        let (client, server, _) = mock_router_connection(MockBehavior::PermissionTrap).await;
        let collector = EndpointCollector::new("test-router", &client, Duration::from_secs(1));
        assert!(matches!(
            collector
                .required_many::<Identity>(PrintCommand::System(System::Identity))
                .await,
            Err(Error::Client(_))
        ));
        let optional = collector
            .optional_many::<Identity>(PrintCommand::System(System::Identity))
            .await;
        assert_eq!(optional.error.unwrap().kind, EndpointErrorKind::PermissionDenied);
        drop(client);
        server.await.unwrap();

        let (client, server, _) = mock_router_connection(MockBehavior::Success).await;
        let collector = EndpointCollector::new("test-router", &client, Duration::from_secs(1));
        assert!(matches!(
            collector
                .required_first::<Identity>(PrintCommand::Interface(InterfaceCommand::Interface))
                .await,
            Err(Error::RequiredEndpointEmpty { .. })
        ));
        let optional = collector
            .optional_first::<Identity>(PrintCommand::Interface(InterfaceCommand::Interface))
            .await;
        assert_eq!(optional.data, Identity::default());
        assert!(optional.error.is_none());
        drop(client);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn endpoint_collector_times_out_required_and_optional_commands() {
        let (client, server, _) = mock_router_connection(MockBehavior::Stall).await;
        let collector = EndpointCollector::new("test-router", &client, Duration::from_millis(1));
        assert!(matches!(
            collector
                .required_many::<Identity>(PrintCommand::System(System::Identity))
                .await,
            Err(Error::CommandTimeout { .. })
        ));
        server.await.unwrap();

        let (client, server, _) = mock_router_connection(MockBehavior::Stall).await;
        let collector = EndpointCollector::new("test-router", &client, Duration::from_millis(1));
        let optional = collector
            .optional_many::<Identity>(PrintCommand::System(System::Identity))
            .await;
        assert_eq!(optional.error.unwrap().kind, EndpointErrorKind::Timeout);
        server.await.unwrap();
    }

    #[test]
    fn retained_target_address_parser_reports_invalid_input() {
        assert_eq!(parse_target_address("192.0.2.1:8728").unwrap(), target().address);
        assert!(matches!(
            parse_target_address("invalid"),
            Err(Error::InvalidTarget { address, .. }) if address == "invalid"
        ));
    }
}
