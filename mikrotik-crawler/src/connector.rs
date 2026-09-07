//! `RouterOS` API connection policy for snapshot collection.

use core::future::Future;
#[cfg(test)]
use core::net::Ipv6Addr;
use core::pin::Pin;
use core::time::Duration;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_client::builder::ClientBuilder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::Client;
use mikrotik_client::error::Error as ClientError;
use mikrotik_common::warn_with_label;
use mikrotik_types::target::DeviceTarget;

use crate::CollectedSnapshot;
use crate::config::DEFAULT_COMMAND_TIMEOUT;
use crate::config::DEFAULT_CONNECT_TIMEOUT;
use crate::error::Error;
use crate::error::Result;
use crate::snapshot::RouterOsApiDiscoveryClient;

/// Boxed future returned by object-safe crawler connector traits.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Connected read-only discovery client.
pub trait DiscoveryClient: Send + Sync {
    /// Collect one read-only discovery snapshot.
    fn snapshot<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<CollectedSnapshot>>;

    /// Collect the small, frequently sampled telemetry subset.
    fn telemetry<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<crate::telemetry::TelemetrySnapshot>> {
        Box::pin(async move {
            let started = Instant::now();
            let snapshot = self.snapshot(target_address).await?;

            Ok(crate::telemetry::TelemetrySnapshot {
                target_address: snapshot.target_address,
                collected_at: snapshot.collected_at,
                collection_duration: started.elapsed(),
                resource: snapshot.snapshot.system.resource.data,
                health: snapshot.snapshot.system.health.data,
                interfaces: snapshot.snapshot.interface.interfaces.data,
            })
        })
    }
}

/// Connector for read-only snapshot clients.
pub trait SnapshotClientConnector: Send + Sync {
    /// Connect to one target.
    fn connect<'a>(&'a self, target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>>;

    /// Connect to one target with per-attempt timeout overrides.
    fn connect_with_timeouts<'a>(
        &'a self,
        target: &'a DeviceTarget,
        _connect_timeout: Duration,
        _command_timeout: Duration,
    ) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
        self.connect(target)
    }
}

/// Default connector backed by `mikrotik-client`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterOsApiConnector {
    /// `RouterOS` API transport protocol.
    protocol: Protocol,
    /// Whether to fall back from `api-ssl` to plaintext `api`.
    fallback_to_api: bool,
    /// Maximum time spent trying to connect to one target.
    connect_timeout: Duration,
    /// Maximum time spent waiting for one print command.
    command_timeout: Duration,
}

impl RouterOsApiConnector {
    /// Build a connector for the requested `RouterOS` API protocol.
    #[must_use]
    pub const fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            fallback_to_api: false,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }

    /// Fall back to plaintext `api` when `api-ssl` cannot be reached.
    #[must_use]
    pub const fn with_api_fallback(mut self) -> Self {
        self.fallback_to_api = true;
        self
    }

    /// Override the maximum time spent trying to connect to one target.
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Override the maximum time spent waiting for one print command.
    #[must_use]
    pub const fn with_command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }
}

impl Default for RouterOsApiConnector {
    fn default() -> Self {
        Self::new(Protocol::ApiSsl).with_api_fallback()
    }
}

impl SnapshotClientConnector for RouterOsApiConnector {
    fn connect<'a>(&'a self, target: &'a DeviceTarget) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
        self.connect_with_timeouts(target, self.connect_timeout, self.command_timeout)
    }

    fn connect_with_timeouts<'a>(
        &'a self,
        target: &'a DeviceTarget,
        connect_timeout: Duration,
        command_timeout: Duration,
    ) -> BoxFuture<'a, Result<Arc<dyn DiscoveryClient>>> {
        Box::pin(async move {
            match self
                .connect_with_protocol(target, self.protocol, connect_timeout, command_timeout)
                .await
            {
                Ok(client) => Ok(client),
                Err(error)
                    if self.fallback_to_api
                        && self.protocol == Protocol::ApiSsl
                        && is_api_ssl_fallback_error(&error) =>
                {
                    warn_with_label!(
                        target.address,
                        "api-ssl connection failed; falling back to api: {error}"
                    );
                    self.connect_with_protocol(target, Protocol::Api, connect_timeout, command_timeout)
                        .await
                }
                Err(error) => Err(error),
            }
        })
    }
}

impl RouterOsApiConnector {
    /// Open one binary API connection using an explicit protocol.
    async fn connect_with_protocol(
        &self,
        target: &DeviceTarget,
        protocol: Protocol,
        connect_timeout: Duration,
        command_timeout: Duration,
    ) -> Result<Arc<dyn DiscoveryClient>> {
        let builder = builder_from_target(target, protocol, connect_timeout);
        let client = Client::connect(builder).await?;
        Ok(Arc::new(RouterOsApiDiscoveryClient {
            client,
            command_timeout,
        }) as Arc<dyn DiscoveryClient>)
    }
}

/// Build a connection builder from a target address.
pub(crate) fn builder_from_target(
    target: &DeviceTarget,
    protocol: Protocol,
    connect_timeout: Duration,
) -> ClientBuilder {
    let host = target.address.ip().to_string();
    let port = target.address.port();
    let port = port_for_protocol_attempt(port, protocol);
    ClientBuilder::new(host, protocol, target.credentials.clone())
        .with_port(port)
        .with_connect_retry_timeout(connect_timeout)
        .with_connect_attempt_timeout(connect_timeout)
}

/// Choose the TCP port for one protocol attempt.
fn port_for_protocol_attempt(port: u16, protocol: Protocol) -> u16 {
    match (port, protocol) {
        (8728, Protocol::ApiSsl) => Protocol::ApiSsl.default_port(),
        (8729, Protocol::Api) => Protocol::Api.default_port(),
        _ => port,
    }
}

/// Return whether an `api-ssl` connection error is safe to retry as `api`.
fn is_api_ssl_fallback_error(error: &Error) -> bool {
    matches!(
        error,
        Error::Client(
            ClientError::Transport { .. }
                | ClientError::Timeout { .. }
                | ClientError::Connection { .. }
                | ClientError::ConnectionClosed { .. }
        ) | Error::Io(_)
    )
}

/// Split a target into host and port, defaulting to the plaintext API port.
#[cfg(test)]
pub(crate) fn split_host_port(address: &str) -> Result<(String, u16)> {
    if address.trim().is_empty() {
        return Err(Error::InvalidTarget {
            address: address.to_owned(),
            message: "address is empty".to_owned(),
        });
    }

    if let Some(stripped) = address.strip_prefix('[') {
        let Some((host, rest)) = stripped.split_once(']') else {
            return Err(Error::InvalidTarget {
                address: address.to_owned(),
                message: "missing closing IPv6 bracket".to_owned(),
            });
        };
        let port = if let Some(port) = rest.strip_prefix(':') {
            parse_port(address, port)?
        } else if rest.is_empty() {
            Protocol::Api.default_port()
        } else {
            return Err(Error::InvalidTarget {
                address: address.to_owned(),
                message: "unexpected data after IPv6 bracket".to_owned(),
            });
        };
        return Ok((host.to_owned(), port));
    }

    if address.parse::<Ipv6Addr>().is_ok() {
        return Ok((address.to_owned(), Protocol::Api.default_port()));
    }

    if let Some((host, port)) = address.rsplit_once(':') {
        if host.is_empty() {
            return Err(Error::InvalidTarget {
                address: address.to_owned(),
                message: "host is empty".to_owned(),
            });
        }
        return Ok((host.to_owned(), parse_port(address, port)?));
    }

    Ok((address.to_owned(), Protocol::Api.default_port()))
}

/// Parse a TCP port.
#[cfg(test)]
fn parse_port(address: &str, port: &str) -> Result<u16> {
    port.parse().map_err(|error| Error::InvalidTarget {
        address: address.to_owned(),
        message: format!("invalid port: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::ErrorKind;

    use mikrotik_types::target::Credentials;
    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;

    use super::*;
    use crate::snapshot::tests::encode_sentence;
    use crate::snapshot::tests::read_sentence;

    fn target(address: &str) -> DeviceTarget {
        DeviceTarget {
            address: address.parse().unwrap(),
            credentials: Credentials {
                username: "observer".to_owned(),
                password: Some("secret".to_owned()),
            },
        }
    }

    #[test]
    fn connector_defaults_and_timeout_builders_are_explicit() {
        let connector = RouterOsApiConnector::default();
        assert_eq!(connector.protocol, Protocol::ApiSsl);
        assert!(connector.fallback_to_api);
        assert_eq!(connector.connect_timeout, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(connector.command_timeout, DEFAULT_COMMAND_TIMEOUT);

        let connector = RouterOsApiConnector::new(Protocol::Api)
            .with_api_fallback()
            .with_connect_timeout(Duration::from_secs(2))
            .with_command_timeout(Duration::from_secs(3));
        assert_eq!(connector.protocol, Protocol::Api);
        assert!(connector.fallback_to_api);
        assert_eq!(connector.connect_timeout, Duration::from_secs(2));
        assert_eq!(connector.command_timeout, Duration::from_secs(3));
    }

    #[test]
    fn target_builder_switches_only_the_standard_api_ports() {
        let plaintext = builder_from_target(&target("192.0.2.1:8729"), Protocol::Api, Duration::from_secs(2));
        assert_eq!(plaintext.socket_address(), "192.0.2.1:8728");
        assert_eq!(plaintext.credentials.username, "observer");
        assert_eq!(plaintext.connect_retry_timeout, Some(Duration::from_secs(2)));

        let tls = builder_from_target(&target("[2001:db8::1]:8728"), Protocol::ApiSsl, Duration::from_secs(3));
        assert_eq!(tls.socket_address(), "[2001:db8::1]:8729");

        let custom = builder_from_target(&target("192.0.2.1:9000"), Protocol::ApiSsl, Duration::from_secs(4));
        assert_eq!(custom.socket_address(), "192.0.2.1:9000");
    }

    #[test]
    fn api_ssl_fallback_is_limited_to_transport_style_failures() {
        for error in [
            Error::from(ClientError::Transport {
                command: None,
                source: io::Error::from(ErrorKind::ConnectionRefused),
            }),
            Error::from(ClientError::Timeout {
                operation: "connect",
                duration: Duration::from_secs(1),
            }),
            Error::from(ClientError::ConnectionClosed { command: None }),
            Error::Io(io::Error::from(ErrorKind::BrokenPipe)),
        ] {
            assert!(is_api_ssl_fallback_error(&error));
        }
        assert!(!is_api_ssl_fallback_error(&Error::from(
            ClientError::UnsupportedProtocol("ssh")
        )));
        assert!(!is_api_ssl_fallback_error(&Error::InvalidTarget {
            address: "invalid".to_owned(),
            message: "bad".to_owned(),
        }));
    }

    #[tokio::test]
    async fn api_ssl_connector_falls_back_to_plaintext_on_the_same_custom_port() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut tls_stream, _) = listener.accept().await.unwrap();
            let mut client_hello = [0; 4096];
            assert_ne!(tls_stream.read(&mut client_hello).await.unwrap(), 0);
            drop(tls_stream);

            let (mut api_stream, _) = listener.accept().await.unwrap();
            let mut buffered = Vec::new();
            let words = read_sentence(&mut api_stream, &mut buffered).await.unwrap();
            assert_eq!(words.first().map(String::as_str), Some("/login"));
            let tag = words.iter().find_map(|word| word.strip_prefix(".tag=")).unwrap();
            let tag_word = format!(".tag={tag}");
            api_stream
                .write_all(&encode_sentence(&[b"!done", tag_word.as_bytes()]))
                .await
                .unwrap();
            let mut trailing = [0; 1];
            assert_eq!(api_stream.read(&mut trailing).await.unwrap(), 0);
        });

        let connector = RouterOsApiConnector::new(Protocol::ApiSsl)
            .with_api_fallback()
            .with_connect_timeout(Duration::from_millis(50));
        let target = target(&address.to_string());
        let connected = connector.connect(&target).await.unwrap();
        drop(connected);
        server.await.unwrap();
    }

    #[test]
    fn split_host_port_supports_ipv4_hostnames_and_ipv6_forms() {
        assert_eq!(split_host_port("192.0.2.1").unwrap(), ("192.0.2.1".to_owned(), 8728));
        assert_eq!(split_host_port("router:9000").unwrap(), ("router".to_owned(), 9000));
        assert_eq!(
            split_host_port("2001:db8::1").unwrap(),
            ("2001:db8::1".to_owned(), 8728)
        );
        assert_eq!(
            split_host_port("[2001:db8::1]").unwrap(),
            ("2001:db8::1".to_owned(), 8728)
        );
        assert_eq!(
            split_host_port("[2001:db8::1]:9000").unwrap(),
            ("2001:db8::1".to_owned(), 9000)
        );
    }

    #[test]
    fn split_host_port_rejects_malformed_brackets_hosts_and_ports() {
        for invalid in ["", "   ", "[2001:db8::1", "[2001:db8::1]extra", ":8728", "router:bad"] {
            assert!(matches!(split_host_port(invalid), Err(Error::InvalidTarget { .. })));
        }
    }
}
