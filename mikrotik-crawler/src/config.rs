//! Runtime configuration for crawler workflows.

use core::net::IpAddr;
use core::time::Duration;

use mikrotik_client::builder::Protocol;
use mikrotik_types::target::DeviceTarget;
use serde::Deserialize;
use serde::Serialize;

/// Default number of extra attempts for timed-out neighbor targets.
pub const DEFAULT_CONNECT_RETRIES: usize = 1;
/// Default maximum time spent trying to connect to one discovered target.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Default maximum time spent waiting for one `RouterOS` print command.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

/// Crawler limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlConfig {
    /// Maximum recursive discovery depth.
    pub max_depth: usize,
    /// Maximum number of devices to collect.
    pub max_devices: usize,
    /// Maximum number of in-flight device snapshot jobs.
    pub max_concurrency: usize,
    /// Number of additional attempts for targets that time out.
    pub connect_retries: usize,
    /// Address family allowed for recursively discovered neighbor targets.
    pub address_family: AddressFamily,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_depth: 1,
            max_devices: 1_000,
            max_concurrency: 16,
            connect_retries: DEFAULT_CONNECT_RETRIES,
            address_family: AddressFamily::Ipv4,
        }
    }
}

/// Address-family filter for recursively discovered neighbor targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressFamily {
    /// Crawl IPv4 and IPv6 neighbor addresses.
    Any,
    /// Crawl only IPv4 neighbor addresses.
    Ipv4,
    /// Crawl only IPv6 neighbor addresses.
    Ipv6,
}

impl AddressFamily {
    /// Return whether an address is allowed by this filter.
    #[must_use]
    pub const fn includes(self, address: IpAddr) -> bool {
        match self {
            Self::Any => true,
            Self::Ipv4 => address.is_ipv4(),
            Self::Ipv6 => address.is_ipv6(),
        }
    }
}

/// Configuration for the long-running crawler service.
#[derive(Debug, Clone)]
pub struct CrawlerServiceConfig {
    /// Initial seed targets.
    pub seeds: Vec<DeviceTarget>,
    /// Maximum number of in-flight snapshot refresh jobs.
    pub snapshot_concurrency: usize,
    /// Interval between target-registry discovery passes.
    pub discovery_interval: Duration,
    /// Interval between snapshot refresh passes.
    pub snapshot_interval: Duration,
    /// Initial TCP connection timeout for a target.
    pub connect_timeout: Duration,
    /// Initial response timeout for one `RouterOS` command.
    pub command_timeout: Duration,
    /// Address family allowed for recursively discovered neighbor targets.
    pub address_family: AddressFamily,
    /// `RouterOS` API transport protocol.
    pub protocol: Protocol,
}

impl CrawlerServiceConfig {
    /// Build service configuration from seed targets.
    #[must_use]
    pub fn new(seeds: Vec<DeviceTarget>) -> Self {
        Self {
            seeds,
            snapshot_concurrency: 4,
            discovery_interval: Duration::from_secs(30),
            snapshot_interval: Duration::from_secs(60),
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            address_family: AddressFamily::Any,
            protocol: Protocol::Api,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::net::IpAddr;

    use mikrotik_types::target::Credentials;

    use super::*;

    #[test]
    fn crawl_defaults_match_documented_limits() {
        let config = CrawlConfig::default();
        assert_eq!(config.max_depth, 1);
        assert_eq!(config.max_devices, 1_000);
        assert_eq!(config.max_concurrency, 16);
        assert_eq!(config.connect_retries, DEFAULT_CONNECT_RETRIES);
        assert_eq!(config.address_family, AddressFamily::Ipv4);
    }

    #[test]
    fn address_family_filters_match_ip_versions() {
        let ipv4 = "192.0.2.1".parse::<IpAddr>().unwrap();
        let ipv6 = "2001:db8::1".parse::<IpAddr>().unwrap();
        assert!(AddressFamily::Any.includes(ipv4));
        assert!(AddressFamily::Any.includes(ipv6));
        assert!(AddressFamily::Ipv4.includes(ipv4));
        assert!(!AddressFamily::Ipv4.includes(ipv6));
        assert!(AddressFamily::Ipv6.includes(ipv6));
        assert!(!AddressFamily::Ipv6.includes(ipv4));
    }

    #[test]
    fn service_config_uses_operational_defaults_and_preserves_seeds() {
        let seed = DeviceTarget {
            address: "192.0.2.1:8728".parse().unwrap(),
            credentials: Credentials {
                username: "admin".to_owned(),
                password: None,
            },
        };
        let config = CrawlerServiceConfig::new(vec![seed.clone()]);
        assert_eq!(config.seeds, [seed]);
        assert_eq!(config.snapshot_concurrency, 4);
        assert_eq!(config.discovery_interval, Duration::from_secs(30));
        assert_eq!(config.snapshot_interval, Duration::from_secs(60));
        assert_eq!(config.connect_timeout, DEFAULT_CONNECT_TIMEOUT);
        assert_eq!(config.command_timeout, DEFAULT_COMMAND_TIMEOUT);
        assert_eq!(config.address_family, AddressFamily::Any);
        assert_eq!(config.protocol, Protocol::Api);
    }
}
