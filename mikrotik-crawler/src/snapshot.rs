//! Device snapshot collection over a connected `RouterOS` API client.

use core::net::SocketAddr;
use core::time::Duration;
use std::collections::BTreeMap;
use std::io::Error as IoError;
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Instant;

use mikrotik_client::client::Client;
use mikrotik_client::commands;
use mikrotik_common::debug_with_label;
use mikrotik_common::info_with_label;
use mikrotik_common::warn_with_label;
use mikrotik_types::api::ip::IpService;
use mikrotik_types::api::service::CapsManAaa;
use mikrotik_types::api::service::CapsManManager;
use mikrotik_types::api::service::CapsManManagerInterface;
use mikrotik_types::api::service::Certificate;
use mikrotik_types::api::service::CertificateSettings;
use mikrotik_types::api::service::ConsoleSettings;
use mikrotik_types::api::service::Disk;
use mikrotik_types::api::service::DiskSettings;
use mikrotik_types::api::service::File;
use mikrotik_types::api::service::MplsSettings;
use mikrotik_types::api::service::Partition;
use mikrotik_types::api::service::PppAaa;
use mikrotik_types::api::service::PppProfile;
use mikrotik_types::api::service::RadiusIncoming;
use mikrotik_types::api::system::Clock;
use mikrotik_types::api::system::Identity;
use mikrotik_types::api::system::Resource;
use mikrotik_types::api::system::Routerboard;
use mikrotik_types::device::EndpointError;
use mikrotik_types::device::EndpointErrorKind;
use mikrotik_types::device::EndpointSnapshot;
use mikrotik_types::device::RouterOsSnapshot;
use mikrotik_types::target::DeviceTarget;
use tokio::time::timeout;

use crate::CollectedSnapshot;
use crate::connector::BoxFuture;
use crate::connector::DiscoveryClient;
use crate::connector::SnapshotClientConnector;
use crate::error::Error;
use crate::error::Result;

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
    #[allow(
        clippy::too_many_lines,
        reason = "the transaction enumerates every typed RouterOS endpoint in collection order"
    )]
    fn snapshot<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<CollectedSnapshot>> {
        Box::pin(async move {
            let identity: Identity = print_first_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Identity),
                self.command_timeout,
            )
            .await?;
            let routerboard: Routerboard = print_first_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Routerboard),
                self.command_timeout,
            )
            .await?;
            let resource: Resource = print_first_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Resource),
                self.command_timeout,
            )
            .await?;
            let clock: EndpointResult<Option<Clock>> = print_first_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Clock),
                self.command_timeout,
            )
            .await?;
            let packages = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Package),
                self.command_timeout,
            )
            .await?;
            let package_updates = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::PackageUpdate),
                self.command_timeout,
            )
            .await?;
            let health = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Health),
                self.command_timeout,
            )
            .await?;
            let resource_cpus = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::ResourceCpu),
                self.command_timeout,
            )
            .await?;
            let resource_hardware = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::ResourceHardware),
                self.command_timeout,
            )
            .await?;
            let resource_irqs = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::ResourceIrq),
                self.command_timeout,
            )
            .await?;
            let resource_usb_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::ResourceUsbSettings),
                self.command_timeout,
            )
            .await?;
            let routerboard_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::RouterboardSettings),
                self.command_timeout,
            )
            .await?;
            let routerboard_reset_buttons = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::RouterboardResetButton),
                self.command_timeout,
            )
            .await?;
            let device_modes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::DeviceMode),
                self.command_timeout,
            )
            .await?;
            let history_entries = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::HistoryEntry),
                self.command_timeout,
            )
            .await?;
            let leds = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Led),
                self.command_timeout,
            )
            .await?;
            let licenses = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::License),
                self.command_timeout,
            )
            .await?;
            let log_entries = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::LogEntry),
                self.command_timeout,
            )
            .await?;
            let logging_rules = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::LoggingRule),
                self.command_timeout,
            )
            .await?;
            let logging_actions = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::LoggingAction),
                self.command_timeout,
            )
            .await?;
            let notes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Note),
                self.command_timeout,
            )
            .await?;
            let ntp_clients = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::NtpClient),
                self.command_timeout,
            )
            .await?;
            let ntp_servers = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::NtpServer),
                self.command_timeout,
            )
            .await?;
            let scripts = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Script),
                self.command_timeout,
            )
            .await?;
            let script_jobs = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::ScriptJob),
                self.command_timeout,
            )
            .await?;
            let schedulers = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Scheduler),
                self.command_timeout,
            )
            .await?;
            let upgrade_mirrors = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::UpgradeMirror),
                self.command_timeout,
            )
            .await?;
            let watchdogs = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Watchdog),
                self.command_timeout,
            )
            .await?;
            let interfaces = print_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::Interface),
                self.command_timeout,
            )
            .await?;
            let ethernet_interfaces = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::EthernetInterface),
                self.command_timeout,
            )
            .await?;
            let bridges = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::Bridge),
                self.command_timeout,
            )
            .await?;
            let bridge_hosts = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::BridgeHost),
                self.command_timeout,
            )
            .await?;
            let bridge_ports = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::BridgePort),
                self.command_timeout,
            )
            .await?;
            let bridge_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::BridgeSettings),
                self.command_timeout,
            )
            .await?;
            let bridge_vlans = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::BridgeVlan),
                self.command_timeout,
            )
            .await?;
            let detect_internet = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::DetectInternet),
                self.command_timeout,
            )
            .await?;
            let ethernet_switches = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::EthernetSwitch),
                self.command_timeout,
            )
            .await?;
            let ethernet_switch_ports = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::EthernetSwitchPort),
                self.command_timeout,
            )
            .await?;
            let ethernet_switch_port_isolations = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::EthernetSwitchPortIsolation),
                self.command_timeout,
            )
            .await?;
            let interface_lists = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::InterfaceList),
                self.command_timeout,
            )
            .await?;
            let interface_list_members = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::InterfaceListMember),
                self.command_timeout,
            )
            .await?;
            let lte_apns = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::LteApn),
                self.command_timeout,
            )
            .await?;
            let vlan_interfaces = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::VlanInterface),
                self.command_timeout,
            )
            .await?;
            let wireguard_interfaces = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::WireGuardInterface),
                self.command_timeout,
            )
            .await?;
            let wireguard_peers = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::WireGuardPeer),
                self.command_timeout,
            )
            .await?;
            let wireless_security_profiles = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::WirelessSecurityProfile),
                self.command_timeout,
            )
            .await?;
            let addresses = print_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Address),
                self.command_timeout,
            )
            .await?;
            let arp_entries = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::ArpEntry),
                self.command_timeout,
            )
            .await?;
            let dhcp_clients = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::DhcpClient),
                self.command_timeout,
            )
            .await?;
            let dhcp_servers = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::DhcpServer),
                self.command_timeout,
            )
            .await?;
            let dhcp_server_networks = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::DhcpServerNetwork),
                self.command_timeout,
            )
            .await?;
            let dhcp_leases = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::DhcpLease),
                self.command_timeout,
            )
            .await?;
            let dns = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Dns),
                self.command_timeout,
            )
            .await?;
            let dns_cache_entries = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::DnsCacheEntry),
                self.command_timeout,
            )
            .await?;
            let neighbors = print_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Neighbor),
                self.command_timeout,
            )
            .await?;
            let routes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Route),
                self.command_timeout,
            )
            .await?;
            let firewall_filter_rules = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallRuleFilter),
                self.command_timeout,
            )
            .await?;
            let firewall_nat_rules = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallRuleNat),
                self.command_timeout,
            )
            .await?;
            let firewall_address_list_entries = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallAddressListEntry),
                self.command_timeout,
            )
            .await?;
            let firewall_connections = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallConnection),
                self.command_timeout,
            )
            .await?;
            let firewall_connection_tracking = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallConnectionTracking),
                self.command_timeout,
            )
            .await?;
            let firewall_mangle_rules = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallRuleMangle),
                self.command_timeout,
            )
            .await?;
            let firewall_raw_rules = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallRuleRaw),
                self.command_timeout,
            )
            .await?;
            let firewall_service_ports = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::FirewallServicePort),
                self.command_timeout,
            )
            .await?;
            let hotspot_profiles = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::HotspotProfile),
                self.command_timeout,
            )
            .await?;
            let hotspot_users = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::HotspotUser),
                self.command_timeout,
            )
            .await?;
            let ip_cloud = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpCloud),
                self.command_timeout,
            )
            .await?;
            let ip_pools = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpPool),
                self.command_timeout,
            )
            .await?;
            let ip_pool_used = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpPoolUsed),
                self.command_timeout,
            )
            .await?;
            let ip_proxy = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpProxy),
                self.command_timeout,
            )
            .await?;
            let ip_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpSettings),
                self.command_timeout,
            )
            .await?;
            let ipsec_policies = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpsecPolicy),
                self.command_timeout,
            )
            .await?;
            let ipsec_profiles = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpsecProfile),
                self.command_timeout,
            )
            .await?;
            let ipsec_proposals = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpsecProposal),
                self.command_timeout,
            )
            .await?;
            let ipsec_statistics = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::IpsecStatistics),
                self.command_timeout,
            )
            .await?;
            let ipv6_addresses = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ipv6Address),
                self.command_timeout,
            )
            .await?;
            let ipv6_neighbors = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ipv6Neighbor),
                self.command_timeout,
            )
            .await?;
            let ipv6_neighbor_discovery = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ipv6NeighborDiscovery),
                self.command_timeout,
            )
            .await?;
            let ipv6_routes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ipv6Route),
                self.command_timeout,
            )
            .await?;
            let ipv6_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ipv6Settings),
                self.command_timeout,
            )
            .await?;
            let nat_pmp = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::NatPmp),
                self.command_timeout,
            )
            .await?;
            let neighbor_discovery_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::NeighborDiscoverySettings),
                self.command_timeout,
            )
            .await?;
            let smb = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Smb),
                self.command_timeout,
            )
            .await?;
            let smb_shares = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::SmbShare),
                self.command_timeout,
            )
            .await?;
            let socks = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Socks),
                self.command_timeout,
            )
            .await?;
            let ssh = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Ssh),
                self.command_timeout,
            )
            .await?;
            let traffic_flow = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::TrafficFlow),
                self.command_timeout,
            )
            .await?;
            let upnp = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Upnp),
                self.command_timeout,
            )
            .await?;
            let vrfs = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Ip(commands::Ip::Vrf),
                self.command_timeout,
            )
            .await?;
            let service_snapshot = collect_service_snapshot(target_address, &self.client, self.command_timeout).await?;
            let queue_interfaces = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Queue(commands::Queue::QueueInterface),
                self.command_timeout,
            )
            .await?;
            let queue_types = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Queue(commands::Queue::QueueType),
                self.command_timeout,
            )
            .await?;
            let snmp = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Snmp(commands::Snmp::Snmp),
                self.command_timeout,
            )
            .await?;
            let snmp_communities = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Snmp(commands::Snmp::SnmpCommunity),
                self.command_timeout,
            )
            .await?;
            let tool_snapshot = collect_tool_snapshot(target_address, &self.client, self.command_timeout).await?;
            let bgp_sessions = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::BgpSession),
                self.command_timeout,
            )
            .await?;
            let bgp_connections = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::BgpConnection),
                self.command_timeout,
            )
            .await?;
            let bgp_peers = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::BgpPeer),
                self.command_timeout,
            )
            .await?;
            let bgp_templates = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::BgpTemplate),
                self.command_timeout,
            )
            .await?;
            let igmp_proxy = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::IgmpProxy),
                self.command_timeout,
            )
            .await?;
            let routing_ids = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingId),
                self.command_timeout,
            )
            .await?;
            let routing_nexthops = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingNexthop),
                self.command_timeout,
            )
            .await?;
            let routing_routes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingRoute),
                self.command_timeout,
            )
            .await?;
            let routing_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingSettings),
                self.command_timeout,
            )
            .await?;
            let routing_stats_memory = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingStatsMemory),
                self.command_timeout,
            )
            .await?;
            let routing_stats_origin = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingStatsOrigin),
                self.command_timeout,
            )
            .await?;
            let routing_stats_processes = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingStatsProcess),
                self.command_timeout,
            )
            .await?;
            let routing_stats_steps = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingStatsStep),
                self.command_timeout,
            )
            .await?;
            let routing_tables = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Routing(commands::Routing::RoutingTable),
                self.command_timeout,
            )
            .await?;
            let active_users = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::ActiveUser),
                self.command_timeout,
            )
            .await?;
            let ssh_keys = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::SshKey),
                self.command_timeout,
            )
            .await?;
            let users = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::User),
                self.command_timeout,
            )
            .await?;
            let user_aaa = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::UserAaa),
                self.command_timeout,
            )
            .await?;
            let user_groups = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::UserGroup),
                self.command_timeout,
            )
            .await?;
            let user_settings = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::User(commands::User::UserSettings),
                self.command_timeout,
            )
            .await?;
            Ok(CollectedSnapshot {
                target_address: target_address
                    .parse::<SocketAddr>()
                    .map_err(|error| Error::InvalidTarget {
                        address: target_address.to_owned(),
                        message: error.to_string(),
                    })?,
                collected_at: time::OffsetDateTime::now_utc(),
                snapshot_duration: Duration::ZERO,
                snapshot: RouterOsSnapshot {
                    system: mikrotik_types::device::SystemSnapshot {
                        identity: (identity).into(),
                        resource: (resource).into(),
                        routerboard: (routerboard).into(),
                        clock: clock.into_value_or_default().into(),
                        packages: (packages).into(),
                        package_updates: (package_updates).into(),
                        health: (health).into(),
                        resource_cpus: (resource_cpus).into(),
                        resource_hardware: (resource_hardware).into(),
                        resource_irqs: (resource_irqs).into(),
                        resource_usb_settings: (resource_usb_settings).into(),
                        routerboard_settings: (routerboard_settings).into(),
                        routerboard_reset_buttons: (routerboard_reset_buttons).into(),
                        device_modes: (device_modes).into(),
                        history_entries: (history_entries).into(),
                        leds: (leds).into(),
                        licenses: (licenses).into(),
                        log_entries: (log_entries).into(),
                        logging_rules: (logging_rules).into(),
                        logging_actions: (logging_actions).into(),
                        notes: (notes).into(),
                        ntp_clients: (ntp_clients).into(),
                        ntp_servers: (ntp_servers).into(),
                        scripts: (scripts).into(),
                        script_jobs: (script_jobs).into(),
                        schedulers: (schedulers).into(),
                        upgrade_mirrors: (upgrade_mirrors).into(),
                        watchdogs: (watchdogs).into(),
                    },
                    interface: mikrotik_types::device::InterfaceSnapshot {
                        interfaces: (interfaces).into(),
                        ethernet_interfaces: (ethernet_interfaces).into(),
                        bridges: (bridges).into(),
                        bridge_hosts: (bridge_hosts).into(),
                        bridge_ports: (bridge_ports).into(),
                        bridge_settings: (bridge_settings).into(),
                        bridge_vlans: (bridge_vlans).into(),
                        detect_internet: (detect_internet).into(),
                        ethernet_switches: (ethernet_switches).into(),
                        ethernet_switch_ports: (ethernet_switch_ports).into(),
                        ethernet_switch_port_isolations: (ethernet_switch_port_isolations).into(),
                        interface_lists: (interface_lists).into(),
                        interface_list_members: (interface_list_members).into(),
                        lte_apns: (lte_apns).into(),
                        vlan_interfaces: (vlan_interfaces).into(),
                        wireguard_interfaces: (wireguard_interfaces).into(),
                        wireguard_peers: (wireguard_peers).into(),
                        wireless_security_profiles: (wireless_security_profiles).into(),
                    },
                    ip: mikrotik_types::device::IpSnapshot {
                        neighbors: (neighbors).into(),
                        addresses: (addresses).into(),
                        arp_entries: (arp_entries).into(),
                        dhcp_clients: (dhcp_clients).into(),
                        dhcp_servers: (dhcp_servers).into(),
                        dhcp_server_networks: (dhcp_server_networks).into(),
                        dhcp_leases: (dhcp_leases).into(),
                        dns: (dns).into(),
                        dns_cache_entries: (dns_cache_entries).into(),
                        routes: (routes).into(),
                        firewall_filter_rules: (firewall_filter_rules).into(),
                        firewall_nat_rules: (firewall_nat_rules).into(),
                        firewall_address_list_entries: (firewall_address_list_entries).into(),
                        firewall_connections: (firewall_connections).into(),
                        firewall_connection_tracking: (firewall_connection_tracking).into(),
                        firewall_mangle_rules: (firewall_mangle_rules).into(),
                        firewall_raw_rules: (firewall_raw_rules).into(),
                        firewall_service_ports: (firewall_service_ports).into(),
                        hotspot_profiles: (hotspot_profiles).into(),
                        hotspot_users: (hotspot_users).into(),
                        ip_cloud: (ip_cloud).into(),
                        ip_pools: (ip_pools).into(),
                        ip_pool_used: (ip_pool_used).into(),
                        ip_proxy: (ip_proxy).into(),
                        ip_services: (service_snapshot.ip_services).into(),
                        ip_settings: (ip_settings).into(),
                        ipsec_policies: (ipsec_policies).into(),
                        ipsec_profiles: (ipsec_profiles).into(),
                        ipsec_proposals: (ipsec_proposals).into(),
                        ipsec_statistics: (ipsec_statistics).into(),
                        nat_pmp: (nat_pmp).into(),
                        neighbor_discovery_settings: (neighbor_discovery_settings).into(),
                        smb: (smb).into(),
                        smb_shares: (smb_shares).into(),
                        socks: (socks).into(),
                        ssh: (ssh).into(),
                        traffic_flow: (traffic_flow).into(),
                        upnp: (upnp).into(),
                        vrfs: (vrfs).into(),
                    },
                    ipv6: mikrotik_types::device::Ipv6Snapshot {
                        ipv6_addresses: (ipv6_addresses).into(),
                        ipv6_neighbors: (ipv6_neighbors).into(),
                        ipv6_neighbor_discovery: (ipv6_neighbor_discovery).into(),
                        ipv6_routes: (ipv6_routes).into(),
                        ipv6_settings: (ipv6_settings).into(),
                    },
                    certificate: mikrotik_types::device::CertificateSnapshot {
                        certificates: (service_snapshot.certificates).into(),
                        certificate_settings: (service_snapshot.certificate_settings).into(),
                    },
                    console: mikrotik_types::device::ConsoleSnapshot {
                        console_settings: (service_snapshot.console_settings).into(),
                    },
                    disk: mikrotik_types::device::DiskSnapshot {
                        disks: (service_snapshot.disks).into(),
                        disk_settings: (service_snapshot.disk_settings).into(),
                    },
                    file: mikrotik_types::device::FileSnapshot {
                        files: (service_snapshot.files).into(),
                    },
                    partitions: mikrotik_types::device::PartitionsSnapshot {
                        partitions: (service_snapshot.partitions).into(),
                    },
                    caps_man: mikrotik_types::device::CapsManSnapshot {
                        caps_man_aaa: (service_snapshot.caps_man_aaa).into(),
                        caps_man_managers: (service_snapshot.caps_man_managers).into(),
                        caps_man_manager_interfaces: (service_snapshot.caps_man_manager_interfaces).into(),
                    },
                    mpls: mikrotik_types::device::MplsSnapshot {
                        mpls_settings: (service_snapshot.mpls_settings).into(),
                    },
                    ppp: mikrotik_types::device::PppSnapshot {
                        ppp_aaa: (service_snapshot.ppp_aaa).into(),
                        ppp_profiles: (service_snapshot.ppp_profiles).into(),
                    },
                    radius: mikrotik_types::device::RadiusSnapshot {
                        radius_incoming: (service_snapshot.radius_incoming).into(),
                    },
                    queue: mikrotik_types::device::QueueSnapshot {
                        queue_interfaces: (queue_interfaces).into(),
                        queue_types: (queue_types).into(),
                    },
                    snmp: mikrotik_types::device::SnmpSnapshot {
                        snmp: (snmp).into(),
                        snmp_communities: (snmp_communities).into(),
                    },
                    tool: mikrotik_types::device::ToolSnapshot {
                        bandwidth_servers: (tool_snapshot.bandwidth_servers).into(),
                        emails: (tool_snapshot.emails).into(),
                        graphing: (tool_snapshot.graphing).into(),
                        mac_server_pings: (tool_snapshot.mac_server_pings).into(),
                        romon: (tool_snapshot.romon).into(),
                        romon_ports: (tool_snapshot.romon_ports).into(),
                        sms: (tool_snapshot.sms).into(),
                        sniffers: (tool_snapshot.sniffers).into(),
                        traffic_generators: (tool_snapshot.traffic_generators).into(),
                        traffic_generator_latency_distributions: (tool_snapshot
                            .traffic_generator_latency_distributions)
                            .into(),
                    },
                    routing: mikrotik_types::device::RoutingSnapshot {
                        bgp_sessions: (bgp_sessions).into(),
                        bgp_connections: (bgp_connections).into(),
                        bgp_peers: (bgp_peers).into(),
                        bgp_templates: (bgp_templates).into(),
                        igmp_proxy: (igmp_proxy).into(),
                        routing_ids: (routing_ids).into(),
                        routing_nexthops: (routing_nexthops).into(),
                        routing_routes: (routing_routes).into(),
                        routing_settings: (routing_settings).into(),
                        routing_stats_memory: (routing_stats_memory).into(),
                        routing_stats_origin: (routing_stats_origin).into(),
                        routing_stats_processes: (routing_stats_processes).into(),
                        routing_stats_steps: (routing_stats_steps).into(),
                        routing_tables: (routing_tables).into(),
                    },
                    user: mikrotik_types::device::UserSnapshot {
                        active_users: (active_users).into(),
                        ssh_keys: (ssh_keys).into(),
                        users: (users).into(),
                        user_aaa: (user_aaa).into(),
                        user_groups: (user_groups).into(),
                        user_settings: (user_settings).into(),
                    },
                    raw: BTreeMap::new(),
                },
            })
        })
    }

    fn telemetry<'a>(&'a self, target_address: &'a str) -> BoxFuture<'a, Result<crate::telemetry::TelemetrySnapshot>> {
        Box::pin(async move {
            let started = Instant::now();
            let resource = print_first_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Resource),
                self.command_timeout,
            )
            .await?;
            let health = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::System(commands::System::Health),
                self.command_timeout,
            )
            .await?;
            let interfaces = print_optional_skipping_trap(
                target_address,
                &self.client,
                commands::PrintCommand::Interface(commands::Interface::Interface),
                self.command_timeout,
            )
            .await?;
            Ok(crate::telemetry::TelemetrySnapshot {
                target_address: target_address
                    .parse::<SocketAddr>()
                    .map_err(|error| Error::InvalidTarget {
                        address: target_address.to_owned(),
                        message: error.to_string(),
                    })?,
                collected_at: time::OffsetDateTime::now_utc(),
                collection_duration: started.elapsed(),
                resource,
                health: health.data,
                interfaces: interfaces.data,
            })
        })
    }
}

/// Service-family state used to audit management access and platform settings.
struct ServiceSnapshot {
    /// `/ip/service/print` rows.
    ip_services: EndpointResult<Vec<IpService>>,
    /// `/certificate/print` rows.
    certificates: EndpointResult<Vec<Certificate>>,
    /// `/certificate/settings/print` rows.
    certificate_settings: EndpointResult<Vec<CertificateSettings>>,
    /// `/console/settings/print` rows.
    console_settings: EndpointResult<Vec<ConsoleSettings>>,
    /// `/disk/print` rows.
    disks: EndpointResult<Vec<Disk>>,
    /// `/disk/settings/print` rows.
    disk_settings: EndpointResult<Vec<DiskSettings>>,
    /// `/file/print` rows.
    files: EndpointResult<Vec<File>>,
    /// `/partitions/print` rows.
    partitions: EndpointResult<Vec<Partition>>,
    /// `/caps-man/aaa/print` rows.
    caps_man_aaa: EndpointResult<Vec<CapsManAaa>>,
    /// `/caps-man/manager/print` rows.
    caps_man_managers: EndpointResult<Vec<CapsManManager>>,
    /// `/caps-man/manager/interface/print` rows.
    caps_man_manager_interfaces: EndpointResult<Vec<CapsManManagerInterface>>,
    /// `/mpls/settings/print` rows.
    mpls_settings: EndpointResult<Vec<MplsSettings>>,
    /// `/ppp/aaa/print` rows.
    ppp_aaa: EndpointResult<Vec<PppAaa>>,
    /// `/ppp/profile/print` rows.
    ppp_profiles: EndpointResult<Vec<PppProfile>>,
    /// `/radius/incoming/print` rows.
    radius_incoming: EndpointResult<Vec<RadiusIncoming>>,
}

/// Tool-family state.
struct ToolSnapshot {
    /// `/tool/bandwidth-server/print` rows.
    bandwidth_servers: EndpointResult<Vec<mikrotik_types::api::tool::BandwidthServer>>,
    /// `/tool/e-mail/print` rows.
    emails: EndpointResult<Vec<mikrotik_types::api::tool::Email>>,
    /// `/tool/graphing/print` rows.
    graphing: EndpointResult<Vec<mikrotik_types::api::tool::Graphing>>,
    /// `/tool/mac-server/ping/print` rows.
    mac_server_pings: EndpointResult<Vec<mikrotik_types::api::tool::MacServerPing>>,
    /// `/tool/romon/print` rows.
    romon: EndpointResult<Vec<mikrotik_types::api::tool::Romon>>,
    /// `/tool/romon/port/print` rows.
    romon_ports: EndpointResult<Vec<mikrotik_types::api::tool::RomonPort>>,
    /// `/tool/sms/print` rows.
    sms: EndpointResult<Vec<mikrotik_types::api::tool::Sms>>,
    /// `/tool/sniffer/print` rows.
    sniffers: EndpointResult<Vec<mikrotik_types::api::tool::Sniffer>>,
    /// `/tool/traffic-generator/print` rows.
    traffic_generators: EndpointResult<Vec<mikrotik_types::api::tool::TrafficGenerator>>,
    /// `/tool/traffic-generator/stats/latency-distribution/print` rows.
    traffic_generator_latency_distributions:
        EndpointResult<Vec<mikrotik_types::api::tool::TrafficGeneratorLatencyDistribution>>,
}

/// Internal endpoint result retaining collection failures until the typed snapshot is assembled.
struct EndpointResult<T> {
    /// Successfully decoded data, or its empty representation after failure.
    data: T,
    /// Endpoint-local failure.
    error: Option<EndpointError>,
}

impl<T> EndpointResult<T> {
    /// Construct a successful endpoint result.
    const fn success(data: T) -> Self {
        Self { data, error: None }
    }
}

impl<T: Default> EndpointResult<Option<T>> {
    /// Preserve collection status while converting an absent row to its default value.
    fn into_value_or_default(self) -> EndpointResult<T> {
        EndpointResult {
            data: self.data.unwrap_or_default(),
            error: self.error,
        }
    }
}

impl<T> From<EndpointResult<T>> for EndpointSnapshot<T> {
    fn from(result: EndpointResult<T>) -> Self {
        Self {
            data: result.data,
            error: result.error,
        }
    }
}

/// Collect service and certificate state used to audit management access.
#[allow(
    clippy::too_many_lines,
    reason = "service collection enumerates the typed endpoints belonging to this RouterOS family"
)]
async fn collect_service_snapshot(
    target_address: &str,
    client: &Client,
    command_timeout: Duration,
) -> Result<ServiceSnapshot> {
    let ip_services = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Ip(commands::Ip::IpService),
        command_timeout,
    )
    .await?;
    let certificates = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::Certificate),
        command_timeout,
    )
    .await?;
    let certificate_settings = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::CertificateSettings),
        command_timeout,
    )
    .await?;
    let console_settings = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::ConsoleSettings),
        command_timeout,
    )
    .await?;
    let disks = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::Disk),
        command_timeout,
    )
    .await?;
    let disk_settings = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::DiskSettings),
        command_timeout,
    )
    .await?;
    let files = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::File),
        command_timeout,
    )
    .await?;
    let partitions = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::Partition),
        command_timeout,
    )
    .await?;
    let caps_man_aaa = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::CapsManAaa),
        command_timeout,
    )
    .await?;
    let caps_man_managers = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::CapsManManager),
        command_timeout,
    )
    .await?;
    let caps_man_manager_interfaces = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::CapsManManagerInterface),
        command_timeout,
    )
    .await?;
    let mpls_settings = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::MplsSettings),
        command_timeout,
    )
    .await?;
    let ppp_aaa = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::PppAaa),
        command_timeout,
    )
    .await?;
    let ppp_profiles = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::PppProfile),
        command_timeout,
    )
    .await?;
    let radius_incoming = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Service(commands::Service::RadiusIncoming),
        command_timeout,
    )
    .await?;
    Ok(ServiceSnapshot {
        ip_services,
        certificates,
        certificate_settings,
        console_settings,
        disks,
        disk_settings,
        files,
        partitions,
        caps_man_aaa,
        caps_man_managers,
        caps_man_manager_interfaces,
        mpls_settings,
        ppp_aaa,
        ppp_profiles,
        radius_incoming,
    })
}

/// Collect `/tool` state.
async fn collect_tool_snapshot(
    target_address: &str,
    client: &Client,
    command_timeout: Duration,
) -> Result<ToolSnapshot> {
    let bandwidth_servers = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::BandwidthServer),
        command_timeout,
    )
    .await?;
    let emails = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::Email),
        command_timeout,
    )
    .await?;
    let graphing = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::Graphing),
        command_timeout,
    )
    .await?;
    let mac_server_pings = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::MacServerPing),
        command_timeout,
    )
    .await?;
    let romon = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::Romon),
        command_timeout,
    )
    .await?;
    let romon_ports = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::RomonPort),
        command_timeout,
    )
    .await?;
    let sms = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::Sms),
        command_timeout,
    )
    .await?;
    let sniffers = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::Sniffer),
        command_timeout,
    )
    .await?;
    let traffic_generators = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::TrafficGenerator),
        command_timeout,
    )
    .await?;
    let traffic_generator_latency_distributions = print_optional_skipping_trap(
        target_address,
        client,
        commands::PrintCommand::Tool(commands::Tool::TrafficGeneratorLatencyDistribution),
        command_timeout,
    )
    .await?;
    Ok(ToolSnapshot {
        bandwidth_servers,
        emails,
        graphing,
        mac_server_pings,
        romon,
        romon_ports,
        sms,
        sniffers,
        traffic_generators,
        traffic_generator_latency_distributions,
    })
}

/// Print a command and return an empty result when `RouterOS` reports that the
/// command is unavailable on this device.
async fn print_skipping_trap<T>(
    target_address: &str,
    client: &Client,
    command: commands::PrintCommand,
    command_timeout: Duration,
) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    debug_with_label!(target_address, "running {command}");
    match timeout(command_timeout, client.print(command)).await {
        Err(_) => Err(Error::Io(IoError::new(
            ErrorKind::TimedOut,
            format!("{command} exceeded {command_timeout:?}"),
        ))),
        Ok(Ok(rows)) => Ok(rows),
        Ok(Err(mikrotik_client::error::Error::Trap(message))) if is_skippable_command_trap(command, &message) => {
            debug_with_label!(target_address, "skipping {command}: RouterOS trap: {message}");
            Ok(Vec::new())
        }
        Ok(Err(mikrotik_client::error::Error::Trap(message))) => {
            warn_with_label!(target_address, "{command} failed: RouterOS trap: {message}");
            Err(Error::CommandTrap {
                command: command.to_string(),
                message,
            })
        }
        Ok(Err(error)) => {
            warn_with_label!(target_address, "{command} failed: {error}");
            Err(error.into())
        }
    }
}

/// Print a command and return an empty result when an optional data source is
/// unavailable or too slow on this device.
async fn print_optional_skipping_trap<T>(
    target_address: &str,
    client: &Client,
    command: commands::PrintCommand,
    command_timeout: Duration,
) -> Result<EndpointResult<Vec<T>>>
where
    T: serde::de::DeserializeOwned,
{
    debug_with_label!(target_address, "running {command}");
    let command_name = command.to_string();
    match timeout(command_timeout, client.print(command)).await {
        Ok(Ok(rows)) => Ok(EndpointResult::success(rows)),
        Err(_) => Ok(endpoint_failure(
            command_name,
            EndpointErrorKind::Timeout,
            format!("command exceeded {command_timeout:?}"),
        )),
        Ok(Err(mikrotik_client::error::Error::Trap(message))) => {
            let kind = classify_trap(command, &message);
            debug_with_label!(target_address, "recording {command} failure: RouterOS trap: {message}");
            Ok(endpoint_failure(command_name, kind, message))
        }
        Ok(Err(error)) => {
            let message = error.to_string();
            warn_with_label!(target_address, "recording {command} failure: {message}");
            Ok(endpoint_failure(command_name, classify_client_error(&message), message))
        }
    }
}

/// Print an optional command expected to have at most one important row.
async fn print_first_optional_skipping_trap<T>(
    target_address: &str,
    client: &Client,
    command: commands::PrintCommand,
    command_timeout: Duration,
) -> Result<EndpointResult<Option<T>>>
where
    T: serde::de::DeserializeOwned,
{
    let result = print_optional_skipping_trap(target_address, client, command, command_timeout).await?;
    Ok(EndpointResult {
        data: result.data.into_iter().next(),
        error: result.error,
    })
}

/// Construct a failed endpoint result with empty data.
fn endpoint_failure<T: Default>(command: String, kind: EndpointErrorKind, message: String) -> EndpointResult<T> {
    EndpointResult {
        data: T::default(),
        error: Some(EndpointError { kind, command, message }),
    }
}

/// Classify a `RouterOS` command trap for UI and metrics consumers.
fn classify_trap(command: commands::PrintCommand, message: &str) -> EndpointErrorKind {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("not enough permissions") || normalized.contains("permission denied") {
        EndpointErrorKind::PermissionDenied
    } else if is_skippable_command_trap(command, message) {
        EndpointErrorKind::Unsupported
    } else {
        EndpointErrorKind::RouterOsTrap
    }
}

/// Classify a non-trap client failure without coupling the public model to client internals.
fn classify_client_error(message: &str) -> EndpointErrorKind {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("decode") || normalized.contains("deserialize") || normalized.contains("invalid type") {
        EndpointErrorKind::Decode
    } else {
        EndpointErrorKind::Transport
    }
}

/// Return whether a trap indicates that the requested command is unavailable on this `RouterOS` version.
fn is_skippable_command_trap(command: commands::PrintCommand, message: &str) -> bool {
    message == "no such command prefix"
        || message.starts_with("no such command or directory")
        || is_skippable_no_such_item_trap(command, message)
        || is_skippable_support_trap(command, message)
}

/// Return whether a `no such item` trap came from a known optional volatile table.
fn is_skippable_no_such_item_trap(command: commands::PrintCommand, message: &str) -> bool {
    matches!(message, "no such item" | "no such item (4)")
        && matches!(command, commands::PrintCommand::Ip(commands::Ip::FirewallConnection))
}

/// Return whether a `MikroTik` support trap is a known optional command failure.
fn is_skippable_support_trap(command: commands::PrintCommand, message: &str) -> bool {
    message.to_ascii_lowercase().contains("contact mikrotik support")
        && matches!(
            command,
            commands::PrintCommand::System(commands::System::RouterboardResetButton)
                | commands::PrintCommand::Interface(commands::Interface::DetectInternet)
                | commands::PrintCommand::Ip(
                    commands::Ip::IpsecPolicy
                        | commands::Ip::IpsecProfile
                        | commands::Ip::IpsecProposal
                        | commands::Ip::IpsecStatistics,
                )
                | commands::PrintCommand::Routing(commands::Routing::RoutingStatsMemory)
        )
}

/// Print a command expected to have at most one important row.
async fn print_first_skipping_trap<T>(
    target_address: &str,
    client: &Client,
    command: commands::PrintCommand,
    command_timeout: Duration,
) -> Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    let mut rows = print_skipping_trap(target_address, client, command, command_timeout).await?;
    Ok(rows.pop().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORT_TRAP: &str = "error - contact MikroTik support and send a supout file (3)";

    #[test]
    fn support_trap_is_skipped_for_known_optional_commands() {
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Interface(commands::Interface::DetectInternet),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Routing(commands::Routing::RoutingStatsMemory),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Ip(commands::Ip::IpsecPolicy),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Ip(commands::Ip::IpsecProfile),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Ip(commands::Ip::IpsecProposal),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Ip(commands::Ip::IpsecStatistics),
            SUPPORT_TRAP,
        ));
        assert!(is_skippable_command_trap(
            commands::PrintCommand::Ip(commands::Ip::IpsecPolicy),
            "error - contact MikroTik support and send a supout file (2)",
        ));
    }

    #[test]
    fn support_trap_is_not_skipped_for_required_commands() {
        assert!(!is_skippable_command_trap(
            commands::PrintCommand::System(commands::System::Identity),
            SUPPORT_TRAP,
        ));
    }

    #[test]
    fn no_such_item_trap_is_skipped_for_firewall_connections() {
        let command = commands::PrintCommand::Ip(commands::Ip::FirewallConnection);
        assert!(is_skippable_command_trap(command, "no such item"));
        assert!(is_skippable_command_trap(command, "no such item (4)"));
    }

    #[test]
    fn no_such_item_trap_is_not_skipped_for_required_commands() {
        let command = commands::PrintCommand::System(commands::System::Identity);
        assert!(!is_skippable_command_trap(command, "no such item"));
        assert!(!is_skippable_command_trap(command, "no such item (4)"));
    }

    #[test]
    fn endpoint_result_preserves_command_failure() {
        let result: EndpointResult<Vec<Identity>> = endpoint_failure(
            "/system/identity/print".to_owned(),
            EndpointErrorKind::Timeout,
            "command exceeded 5s".to_owned(),
        );
        let snapshot: EndpointSnapshot<Vec<Identity>> = result.into();

        assert!(snapshot.data.is_empty());
        assert_eq!(
            snapshot.error.as_ref().map(|error| error.kind),
            Some(EndpointErrorKind::Timeout)
        );
        assert_eq!(
            snapshot.error.as_ref().map(|error| error.command.as_str()),
            Some("/system/identity/print")
        );
    }

    #[test]
    fn endpoint_traps_are_classified() {
        let command = commands::PrintCommand::Interface(commands::Interface::DetectInternet);
        assert_eq!(classify_trap(command, SUPPORT_TRAP), EndpointErrorKind::Unsupported);
        assert_eq!(
            classify_trap(command, "not enough permissions (9)"),
            EndpointErrorKind::PermissionDenied
        );
        assert_eq!(
            classify_trap(command, "operation failed (1)"),
            EndpointErrorKind::RouterOsTrap
        );
    }
}
