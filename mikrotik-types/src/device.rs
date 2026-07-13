//! Observer-level device snapshots.
//!
//! These types are composed from lower-level `RouterOS` endpoint rows and
//! represent an observed device as a single domain object. They are intentionally
//! separate from raw endpoint structs so collection clients can keep API shape
//! changes isolated from higher-level topology and inventory logic.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString as _;
use alloc::vec::Vec;
use core::fmt;
use core::net::IpAddr;
use core::net::SocketAddr;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use time::OffsetDateTime;

use crate::ParseError;
use crate::Row;
use crate::api::interface::Bridge;
use crate::api::interface::BridgeHost;
use crate::api::interface::BridgePort;
use crate::api::interface::BridgeSettings;
use crate::api::interface::BridgeVlan;
use crate::api::interface::DetectInternet;
use crate::api::interface::EthernetInterface;
use crate::api::interface::EthernetSwitch;
use crate::api::interface::EthernetSwitchPort;
use crate::api::interface::EthernetSwitchPortIsolation;
use crate::api::interface::Interface;
use crate::api::interface::InterfaceList;
use crate::api::interface::InterfaceListMember;
use crate::api::interface::LteApn;
use crate::api::interface::VlanInterface;
use crate::api::interface::WireGuardInterface;
use crate::api::interface::WireGuardPeer;
use crate::api::interface::WirelessSecurityProfile;
use crate::api::ip::Address;
use crate::api::ip::ArpEntry;
use crate::api::ip::DhcpClient;
use crate::api::ip::DhcpLease;
use crate::api::ip::DhcpServer;
use crate::api::ip::DhcpServerNetwork;
use crate::api::ip::Dns;
use crate::api::ip::DnsCacheEntry;
use crate::api::ip::FirewallAddressListEntry;
use crate::api::ip::FirewallConnection;
use crate::api::ip::FirewallConnectionTracking;
use crate::api::ip::FirewallRule;
use crate::api::ip::FirewallServicePort;
use crate::api::ip::HotspotProfile;
use crate::api::ip::HotspotUser;
use crate::api::ip::IpCloud;
use crate::api::ip::IpPool;
use crate::api::ip::IpPoolUsed;
use crate::api::ip::IpProxy;
use crate::api::ip::IpService;
use crate::api::ip::IpSettings;
use crate::api::ip::IpsecPolicy;
use crate::api::ip::IpsecProfile;
use crate::api::ip::IpsecProposal;
use crate::api::ip::IpsecStatistics;
use crate::api::ip::Ipv6Address;
use crate::api::ip::Ipv6Neighbor;
use crate::api::ip::Ipv6NeighborDiscovery;
use crate::api::ip::Ipv6Route;
use crate::api::ip::Ipv6Settings;
use crate::api::ip::NatPmp;
use crate::api::ip::Neighbor;
use crate::api::ip::NeighborDiscoverySettings;
use crate::api::ip::Route;
use crate::api::ip::Smb;
use crate::api::ip::SmbShare;
use crate::api::ip::Socks;
use crate::api::ip::Ssh;
use crate::api::ip::TrafficFlow;
use crate::api::ip::Upnp;
use crate::api::ip::Vrf;
use crate::api::queue::QueueInterface;
use crate::api::queue::QueueType;
use crate::api::routing::BgpConnection;
use crate::api::routing::BgpPeer;
use crate::api::routing::BgpSession;
use crate::api::routing::BgpTemplate;
use crate::api::routing::IgmpProxy;
use crate::api::routing::RoutingId;
use crate::api::routing::RoutingNexthop;
use crate::api::routing::RoutingRoute;
use crate::api::routing::RoutingSettings;
use crate::api::routing::RoutingStatsMemory;
use crate::api::routing::RoutingStatsOrigin;
use crate::api::routing::RoutingStatsProcess;
use crate::api::routing::RoutingStatsStep;
use crate::api::routing::RoutingTable;
use crate::api::service::CapsManAaa;
use crate::api::service::CapsManManager;
use crate::api::service::CapsManManagerInterface;
use crate::api::service::Certificate;
use crate::api::service::CertificateSettings;
use crate::api::service::ConsoleSettings;
use crate::api::service::Disk;
use crate::api::service::DiskSettings;
use crate::api::service::File;
use crate::api::service::MplsSettings;
use crate::api::service::Partition;
use crate::api::service::PppAaa;
use crate::api::service::PppProfile;
use crate::api::service::RadiusIncoming;
use crate::api::snmp::Snmp;
use crate::api::snmp::SnmpCommunity;
use crate::api::system::Clock;
use crate::api::system::DeviceMode;
use crate::api::system::Health;
use crate::api::system::HistoryEntry;
use crate::api::system::Identity;
use crate::api::system::Led;
use crate::api::system::License;
use crate::api::system::LogEntry;
use crate::api::system::LoggingAction;
use crate::api::system::LoggingRule;
use crate::api::system::Note;
use crate::api::system::NtpClient;
use crate::api::system::NtpServer;
use crate::api::system::Package;
use crate::api::system::PackageUpdate;
use crate::api::system::Resource;
use crate::api::system::ResourceCpu;
use crate::api::system::ResourceHardware;
use crate::api::system::ResourceIrq;
use crate::api::system::ResourceUsbSettings;
use crate::api::system::Routerboard;
use crate::api::system::RouterboardResetButton;
use crate::api::system::RouterboardSettings;
use crate::api::system::Scheduler;
use crate::api::system::Script;
use crate::api::system::ScriptJob;
use crate::api::system::UpgradeMirror;
use crate::api::system::Watchdog;
use crate::api::tool::BandwidthServer;
use crate::api::tool::Email;
use crate::api::tool::Graphing;
use crate::api::tool::MacServerPing;
use crate::api::tool::Romon;
use crate::api::tool::RomonPort;
use crate::api::tool::Sms;
use crate::api::tool::Sniffer;
use crate::api::tool::TrafficGenerator;
use crate::api::tool::TrafficGeneratorLatencyDistribution;
use crate::api::user::ActiveUser;
use crate::api::user::SshKey;
use crate::api::user::User;
use crate::api::user::UserAaa;
use crate::api::user::UserGroup;
use crate::api::user::UserSettings;

/// Stable observer key for a device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceKey(String);

impl DeviceKey {
    /// Return the key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DeviceKey {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        crate::parse_non_empty(value).map(Self)
    }
}

impl From<String> for DeviceKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Device reachability state from the observer point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    /// `RouterOS` API connection and collection succeeded.
    Reachable,
    /// The device could not be reached.
    Unreachable,
    /// `RouterOS` authentication failed.
    AuthFailed,
    /// The target is reachable but not supported by this collector.
    Unsupported,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Reachable => "reachable",
            Self::Unreachable => "unreachable",
            Self::AuthFailed => "auth_failed",
            Self::Unsupported => "unsupported",
        })
    }
}

impl FromStr for DeviceStatus {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "reachable" => Ok(Self::Reachable),
            "unreachable" => Ok(Self::Unreachable),
            "auth_failed" => Ok(Self::AuthFailed),
            "unsupported" => Ok(Self::Unsupported),
            _ => Err(ParseError::DeviceStatus),
        }
    }
}

/// Operator-assigned device role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRole {
    /// Role has not been assigned.
    Unknown,
    /// BGP edge or route server.
    BgpRouter,
    /// Core router.
    CoreRouter,
    /// Customer router.
    CustomerRouter,
    /// Switch.
    Switch,
    /// Radio.
    Radio,
}

impl fmt::Display for DeviceRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unknown => "unknown",
            Self::BgpRouter => "bgp_router",
            Self::CoreRouter => "core_router",
            Self::CustomerRouter => "customer_router",
            Self::Switch => "switch",
            Self::Radio => "radio",
        })
    }
}

impl FromStr for DeviceRole {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "unknown" => Ok(Self::Unknown),
            "bgp_router" => Ok(Self::BgpRouter),
            "core_router" => Ok(Self::CoreRouter),
            "customer_router" => Ok(Self::CustomerRouter),
            "switch" => Ok(Self::Switch),
            "radio" => Ok(Self::Radio),
            _ => Err(ParseError::DeviceRole),
        }
    }
}

/// A point-in-time snapshot of one `RouterOS` device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    /// Address used to collect this snapshot.
    pub target_address: SocketAddr,
    /// Collection timestamp.
    pub collected_at: OffsetDateTime,
    /// Collection status.
    pub status: DeviceStatus,
    /// Operator-assigned role.
    pub role: DeviceRole,
    /// Whether `RouterBOARD` firmware has an available pending update.
    #[serde(default)]
    pub fw_update_pending: bool,
    /// Known management or interface addresses for this device.
    #[serde(default)]
    pub management_addresses: Vec<IpAddr>,
    /// `/system/identity/print` row.
    pub identity: Identity,
    /// `/system/resource/print` row.
    pub resource: Resource,
    /// `/system/routerboard/print` row.
    pub routerboard: Routerboard,
    /// `/system/clock/print` row.
    pub clock: Clock,
    /// `/system/package/print` rows.
    pub packages: Vec<Package>,
    /// `/system/package/update/print` rows.
    pub package_updates: Vec<PackageUpdate>,
    /// `/system/health/print` rows.
    pub health: Vec<Health>,
    /// `/system/resource/cpu/print` rows.
    pub resource_cpus: Vec<ResourceCpu>,
    /// `/system/resource/hardware/print` rows.
    pub resource_hardware: Vec<ResourceHardware>,
    /// `/system/resource/irq/print` rows.
    pub resource_irqs: Vec<ResourceIrq>,
    /// `/system/resource/usb/settings/print` rows.
    pub resource_usb_settings: Vec<ResourceUsbSettings>,
    /// `/system/routerboard/settings/print` rows.
    pub routerboard_settings: Vec<RouterboardSettings>,
    /// `/system/routerboard/reset-button/print` rows.
    pub routerboard_reset_buttons: Vec<RouterboardResetButton>,
    /// `/system/device-mode/print` rows.
    pub device_modes: Vec<DeviceMode>,
    /// `/system/history/print` rows.
    pub history_entries: Vec<HistoryEntry>,
    /// `/system/leds/print` rows.
    pub leds: Vec<Led>,
    /// `/system/license/print` rows.
    pub licenses: Vec<License>,
    /// `/log/print` rows.
    pub log_entries: Vec<LogEntry>,
    /// `/system/logging/print` rows.
    pub logging_rules: Vec<LoggingRule>,
    /// `/system/logging/action/print` rows.
    pub logging_actions: Vec<LoggingAction>,
    /// `/system/note/print` rows.
    pub notes: Vec<Note>,
    /// `/system/ntp/client/print` rows.
    pub ntp_clients: Vec<NtpClient>,
    /// `/system/ntp/server/print` rows.
    pub ntp_servers: Vec<NtpServer>,
    /// `/system/script/print` rows.
    pub scripts: Vec<Script>,
    /// `/system/script/job/print` rows.
    pub script_jobs: Vec<ScriptJob>,
    /// `/system/scheduler/print` rows.
    pub schedulers: Vec<Scheduler>,
    /// `/system/upgrade/mirror/print` rows.
    pub upgrade_mirrors: Vec<UpgradeMirror>,
    /// `/system/watchdog/print` rows.
    pub watchdogs: Vec<Watchdog>,
    /// `/interface/print` rows.
    pub interfaces: Vec<Interface>,
    /// `/interface/ethernet/print` rows.
    pub ethernet_interfaces: Vec<EthernetInterface>,
    /// `/interface/bridge/print` rows.
    pub bridges: Vec<Bridge>,
    /// `/interface/bridge/host/print` rows.
    pub bridge_hosts: Vec<BridgeHost>,
    /// `/interface/bridge/port/print` rows.
    pub bridge_ports: Vec<BridgePort>,
    /// `/interface/bridge/settings/print` rows.
    pub bridge_settings: Vec<BridgeSettings>,
    /// `/interface/bridge/vlan/print` rows.
    pub bridge_vlans: Vec<BridgeVlan>,
    /// `/interface/detect-internet/print` rows.
    pub detect_internet: Vec<DetectInternet>,
    /// `/interface/ethernet/switch/print` rows.
    pub ethernet_switches: Vec<EthernetSwitch>,
    /// `/interface/ethernet/switch/port/print` rows.
    pub ethernet_switch_ports: Vec<EthernetSwitchPort>,
    /// `/interface/ethernet/switch/port-isolation/print` rows.
    pub ethernet_switch_port_isolations: Vec<EthernetSwitchPortIsolation>,
    /// `/interface/list/print` rows.
    pub interface_lists: Vec<InterfaceList>,
    /// `/interface/list/member/print` rows.
    pub interface_list_members: Vec<InterfaceListMember>,
    /// `/interface/lte/apn/print` rows.
    pub lte_apns: Vec<LteApn>,
    /// `/interface/vlan/print` rows.
    pub vlan_interfaces: Vec<VlanInterface>,
    /// `/interface/wireguard/print` rows.
    pub wireguard_interfaces: Vec<WireGuardInterface>,
    /// `/interface/wireguard/peers/print` rows.
    pub wireguard_peers: Vec<WireGuardPeer>,
    /// `/interface/wireless/security-profiles/print` rows.
    pub wireless_security_profiles: Vec<WirelessSecurityProfile>,
    /// `/ip/neighbor/print` rows.
    pub neighbors: Vec<Neighbor>,
    /// `/ip/address/print` rows.
    pub addresses: Vec<Address>,
    /// `/ip/arp/print` rows.
    pub arp_entries: Vec<ArpEntry>,
    /// `/ip/dhcp-client/print` rows.
    pub dhcp_clients: Vec<DhcpClient>,
    /// `/ip/dhcp-server/print` rows.
    pub dhcp_servers: Vec<DhcpServer>,
    /// `/ip/dhcp-server/network/print` rows.
    pub dhcp_server_networks: Vec<DhcpServerNetwork>,
    /// `/ip/dhcp-server/lease/print` rows.
    pub dhcp_leases: Vec<DhcpLease>,
    /// `/ip/dns/print` rows.
    pub dns: Vec<Dns>,
    /// `/ip/dns/cache/print` rows.
    pub dns_cache_entries: Vec<DnsCacheEntry>,
    /// `/ip/route/print` rows.
    pub routes: Vec<Route>,
    /// `/ip/firewall/filter/print` rows.
    pub firewall_filter_rules: Vec<FirewallRule>,
    /// `/ip/firewall/nat/print` rows.
    pub firewall_nat_rules: Vec<FirewallRule>,
    /// `/ip/firewall/address-list/print` rows.
    pub firewall_address_list_entries: Vec<FirewallAddressListEntry>,
    /// `/ip/firewall/connection/print` rows.
    pub firewall_connections: Vec<FirewallConnection>,
    /// `/ip/firewall/connection/tracking/print` rows.
    pub firewall_connection_tracking: Vec<FirewallConnectionTracking>,
    /// `/ip/firewall/mangle/print` rows.
    pub firewall_mangle_rules: Vec<FirewallRule>,
    /// `/ip/firewall/raw/print` rows.
    pub firewall_raw_rules: Vec<FirewallRule>,
    /// `/ip/firewall/service-port/print` rows.
    pub firewall_service_ports: Vec<FirewallServicePort>,
    /// `/ip/hotspot/profile/print` rows.
    pub hotspot_profiles: Vec<HotspotProfile>,
    /// `/ip/hotspot/user/print` rows.
    pub hotspot_users: Vec<HotspotUser>,
    /// `/ip/cloud/print` rows.
    pub ip_cloud: Vec<IpCloud>,
    /// `/ip/pool/print` rows.
    pub ip_pools: Vec<IpPool>,
    /// `/ip/pool/used/print` rows.
    pub ip_pool_used: Vec<IpPoolUsed>,
    /// `/ip/proxy/print` rows.
    pub ip_proxy: Vec<IpProxy>,
    /// `/ip/service/print` rows.
    pub ip_services: Vec<IpService>,
    /// `/ip/settings/print` rows.
    pub ip_settings: Vec<IpSettings>,
    /// `/ip/ipsec/policy/print` rows.
    pub ipsec_policies: Vec<IpsecPolicy>,
    /// `/ip/ipsec/profile/print` rows.
    pub ipsec_profiles: Vec<IpsecProfile>,
    /// `/ip/ipsec/proposal/print` rows.
    pub ipsec_proposals: Vec<IpsecProposal>,
    /// `/ip/ipsec/statistics/print` rows.
    pub ipsec_statistics: Vec<IpsecStatistics>,
    /// `/ipv6/address/print` rows.
    pub ipv6_addresses: Vec<Ipv6Address>,
    /// `/ipv6/neighbor/print` rows.
    pub ipv6_neighbors: Vec<Ipv6Neighbor>,
    /// `/ipv6/nd/print` rows.
    pub ipv6_neighbor_discovery: Vec<Ipv6NeighborDiscovery>,
    /// `/ipv6/route/print` rows.
    pub ipv6_routes: Vec<Ipv6Route>,
    /// `/ipv6/settings/print` rows.
    pub ipv6_settings: Vec<Ipv6Settings>,
    /// `/ip/nat-pmp/print` rows.
    pub nat_pmp: Vec<NatPmp>,
    /// `/ip/neighbor/discovery-settings/print` rows.
    pub neighbor_discovery_settings: Vec<NeighborDiscoverySettings>,
    /// `/ip/smb/print` rows.
    pub smb: Vec<Smb>,
    /// `/ip/smb/shares/print` rows.
    pub smb_shares: Vec<SmbShare>,
    /// `/ip/socks/print` rows.
    pub socks: Vec<Socks>,
    /// `/ip/ssh/print` rows.
    pub ssh: Vec<Ssh>,
    /// `/ip/traffic-flow/print` rows.
    pub traffic_flow: Vec<TrafficFlow>,
    /// `/ip/upnp/print` rows.
    pub upnp: Vec<Upnp>,
    /// `/ip/vrf/print` rows.
    pub vrfs: Vec<Vrf>,
    /// `/certificate/print` rows.
    pub certificates: Vec<Certificate>,
    /// `/certificate/settings/print` rows.
    pub certificate_settings: Vec<CertificateSettings>,
    /// `/console/settings/print` rows.
    pub console_settings: Vec<ConsoleSettings>,
    /// `/disk/print` rows.
    pub disks: Vec<Disk>,
    /// `/disk/settings/print` rows.
    pub disk_settings: Vec<DiskSettings>,
    /// `/file/print` rows.
    pub files: Vec<File>,
    /// `/partitions/print` rows.
    pub partitions: Vec<Partition>,
    /// `/caps-man/aaa/print` rows.
    pub caps_man_aaa: Vec<CapsManAaa>,
    /// `/caps-man/manager/print` rows.
    pub caps_man_managers: Vec<CapsManManager>,
    /// `/caps-man/manager/interface/print` rows.
    pub caps_man_manager_interfaces: Vec<CapsManManagerInterface>,
    /// `/mpls/settings/print` rows.
    pub mpls_settings: Vec<MplsSettings>,
    /// `/ppp/aaa/print` rows.
    pub ppp_aaa: Vec<PppAaa>,
    /// `/ppp/profile/print` rows.
    pub ppp_profiles: Vec<PppProfile>,
    /// `/radius/incoming/print` rows.
    pub radius_incoming: Vec<RadiusIncoming>,
    /// `/queue/interface/print` rows.
    pub queue_interfaces: Vec<QueueInterface>,
    /// `/queue/type/print` rows.
    pub queue_types: Vec<QueueType>,
    /// `/snmp/print` rows.
    pub snmp: Vec<Snmp>,
    /// `/snmp/community/print` rows.
    pub snmp_communities: Vec<SnmpCommunity>,
    /// `/tool/bandwidth-server/print` rows.
    pub bandwidth_servers: Vec<BandwidthServer>,
    /// `/tool/e-mail/print` rows.
    pub emails: Vec<Email>,
    /// `/tool/graphing/print` rows.
    pub graphing: Vec<Graphing>,
    /// `/tool/mac-server/ping/print` rows.
    pub mac_server_pings: Vec<MacServerPing>,
    /// `/tool/romon/print` rows.
    pub romon: Vec<Romon>,
    /// `/tool/romon/port/print` rows.
    pub romon_ports: Vec<RomonPort>,
    /// `/tool/sms/print` rows.
    pub sms: Vec<Sms>,
    /// `/tool/sniffer/print` rows.
    pub sniffers: Vec<Sniffer>,
    /// `/tool/traffic-generator/print` rows.
    pub traffic_generators: Vec<TrafficGenerator>,
    /// `/tool/traffic-generator/stats/latency-distribution/print` rows.
    pub traffic_generator_latency_distributions: Vec<TrafficGeneratorLatencyDistribution>,
    /// `/routing/bgp/session/print` rows.
    pub bgp_sessions: Vec<BgpSession>,
    /// `/routing/bgp/connection/print` rows.
    pub bgp_connections: Vec<BgpConnection>,
    /// `RouterOS` v6 `/routing/bgp/peer/print` rows.
    pub bgp_peers: Vec<BgpPeer>,
    /// `/routing/bgp/template/print` rows.
    pub bgp_templates: Vec<BgpTemplate>,
    /// `/routing/igmp-proxy/print` rows.
    pub igmp_proxy: Vec<IgmpProxy>,
    /// `/routing/id/print` rows.
    pub routing_ids: Vec<RoutingId>,
    /// `/routing/nexthop/print` rows.
    pub routing_nexthops: Vec<RoutingNexthop>,
    /// `/routing/route/print` rows.
    pub routing_routes: Vec<RoutingRoute>,
    /// `/routing/settings/print` rows.
    pub routing_settings: Vec<RoutingSettings>,
    /// `/routing/stats/memory/print` rows.
    pub routing_stats_memory: Vec<RoutingStatsMemory>,
    /// `/routing/stats/origin/print` rows.
    pub routing_stats_origin: Vec<RoutingStatsOrigin>,
    /// `/routing/stats/process/print` rows.
    pub routing_stats_processes: Vec<RoutingStatsProcess>,
    /// `/routing/stats/step/print` rows.
    pub routing_stats_steps: Vec<RoutingStatsStep>,
    /// `/routing/table/print` rows.
    pub routing_tables: Vec<RoutingTable>,
    /// `/user/active/print` rows.
    pub active_users: Vec<ActiveUser>,
    /// `/user/ssh-keys/print` rows.
    pub ssh_keys: Vec<SshKey>,
    /// `/user/print` rows.
    pub users: Vec<User>,
    /// `/user/aaa/print` rows.
    pub user_aaa: Vec<UserAaa>,
    /// `/user/group/print` rows.
    pub user_groups: Vec<UserGroup>,
    /// `/user/settings/print` rows.
    pub user_settings: Vec<UserSettings>,
    /// Raw `RouterOS` rows by endpoint name.
    pub raw: BTreeMap<String, Vec<Row>>,
}

impl DeviceSnapshot {
    /// Stable-ish key used before a real inventory identity model exists.
    #[must_use]
    pub fn stable_key(&self) -> DeviceKey {
        self.routerboard
            .serial_number
            .as_deref()
            .or(self.identity.name.as_deref())
            .map_or_else(|| self.target_address.to_string(), alloc::borrow::ToOwned::to_owned)
            .into()
    }

    /// Return whether `RouterBOARD` current and upgrade firmware differ.
    #[must_use]
    pub fn routerboard_fw_update_pending(routerboard: &Routerboard) -> bool {
        routerboard
            .current_firmware
            .as_ref()
            .zip(routerboard.upgrade_firmware.as_ref())
            .is_some_and(|(current, upgrade)| current != upgrade)
    }
}

impl Default for DeviceSnapshot {
    #[allow(clippy::too_many_lines)]
    fn default() -> Self {
        Self {
            target_address: SocketAddr::from(([0, 0, 0, 0], 0)),
            collected_at: OffsetDateTime::UNIX_EPOCH,
            status: DeviceStatus::Unreachable,
            role: DeviceRole::Unknown,
            fw_update_pending: false,
            management_addresses: Vec::new(),
            identity: Identity::default(),
            resource: Resource::default(),
            routerboard: Routerboard::default(),
            clock: Clock::default(),
            packages: Vec::new(),
            package_updates: Vec::new(),
            health: Vec::new(),
            resource_cpus: Vec::new(),
            resource_hardware: Vec::new(),
            resource_irqs: Vec::new(),
            resource_usb_settings: Vec::new(),
            routerboard_settings: Vec::new(),
            routerboard_reset_buttons: Vec::new(),
            device_modes: Vec::new(),
            history_entries: Vec::new(),
            leds: Vec::new(),
            licenses: Vec::new(),
            log_entries: Vec::new(),
            logging_rules: Vec::new(),
            logging_actions: Vec::new(),
            notes: Vec::new(),
            ntp_clients: Vec::new(),
            ntp_servers: Vec::new(),
            scripts: Vec::new(),
            script_jobs: Vec::new(),
            schedulers: Vec::new(),
            upgrade_mirrors: Vec::new(),
            watchdogs: Vec::new(),
            interfaces: Vec::new(),
            ethernet_interfaces: Vec::new(),
            bridges: Vec::new(),
            bridge_hosts: Vec::new(),
            bridge_ports: Vec::new(),
            bridge_settings: Vec::new(),
            bridge_vlans: Vec::new(),
            detect_internet: Vec::new(),
            ethernet_switches: Vec::new(),
            ethernet_switch_ports: Vec::new(),
            ethernet_switch_port_isolations: Vec::new(),
            interface_lists: Vec::new(),
            interface_list_members: Vec::new(),
            lte_apns: Vec::new(),
            vlan_interfaces: Vec::new(),
            wireguard_interfaces: Vec::new(),
            wireguard_peers: Vec::new(),
            wireless_security_profiles: Vec::new(),
            neighbors: Vec::new(),
            addresses: Vec::new(),
            arp_entries: Vec::new(),
            dhcp_clients: Vec::new(),
            dhcp_servers: Vec::new(),
            dhcp_server_networks: Vec::new(),
            dhcp_leases: Vec::new(),
            dns: Vec::new(),
            dns_cache_entries: Vec::new(),
            routes: Vec::new(),
            firewall_filter_rules: Vec::new(),
            firewall_nat_rules: Vec::new(),
            firewall_address_list_entries: Vec::new(),
            firewall_connections: Vec::new(),
            firewall_connection_tracking: Vec::new(),
            firewall_mangle_rules: Vec::new(),
            firewall_raw_rules: Vec::new(),
            firewall_service_ports: Vec::new(),
            hotspot_profiles: Vec::new(),
            hotspot_users: Vec::new(),
            ip_cloud: Vec::new(),
            ip_pools: Vec::new(),
            ip_pool_used: Vec::new(),
            ip_proxy: Vec::new(),
            ip_services: Vec::new(),
            ip_settings: Vec::new(),
            ipsec_policies: Vec::new(),
            ipsec_profiles: Vec::new(),
            ipsec_proposals: Vec::new(),
            ipsec_statistics: Vec::new(),
            ipv6_addresses: Vec::new(),
            ipv6_neighbors: Vec::new(),
            ipv6_neighbor_discovery: Vec::new(),
            ipv6_routes: Vec::new(),
            ipv6_settings: Vec::new(),
            nat_pmp: Vec::new(),
            neighbor_discovery_settings: Vec::new(),
            smb: Vec::new(),
            smb_shares: Vec::new(),
            socks: Vec::new(),
            ssh: Vec::new(),
            traffic_flow: Vec::new(),
            upnp: Vec::new(),
            vrfs: Vec::new(),
            certificates: Vec::new(),
            certificate_settings: Vec::new(),
            console_settings: Vec::new(),
            disks: Vec::new(),
            disk_settings: Vec::new(),
            files: Vec::new(),
            partitions: Vec::new(),
            caps_man_aaa: Vec::new(),
            caps_man_managers: Vec::new(),
            caps_man_manager_interfaces: Vec::new(),
            mpls_settings: Vec::new(),
            ppp_aaa: Vec::new(),
            ppp_profiles: Vec::new(),
            radius_incoming: Vec::new(),
            queue_interfaces: Vec::new(),
            queue_types: Vec::new(),
            snmp: Vec::new(),
            snmp_communities: Vec::new(),
            bandwidth_servers: Vec::new(),
            emails: Vec::new(),
            graphing: Vec::new(),
            mac_server_pings: Vec::new(),
            romon: Vec::new(),
            romon_ports: Vec::new(),
            sms: Vec::new(),
            sniffers: Vec::new(),
            traffic_generators: Vec::new(),
            traffic_generator_latency_distributions: Vec::new(),
            bgp_sessions: Vec::new(),
            bgp_connections: Vec::new(),
            bgp_peers: Vec::new(),
            bgp_templates: Vec::new(),
            igmp_proxy: Vec::new(),
            routing_ids: Vec::new(),
            routing_nexthops: Vec::new(),
            routing_routes: Vec::new(),
            routing_settings: Vec::new(),
            routing_stats_memory: Vec::new(),
            routing_stats_origin: Vec::new(),
            routing_stats_processes: Vec::new(),
            routing_stats_steps: Vec::new(),
            routing_tables: Vec::new(),
            active_users: Vec::new(),
            ssh_keys: Vec::new(),
            users: Vec::new(),
            user_aaa: Vec::new(),
            user_groups: Vec::new(),
            user_settings: Vec::new(),
            raw: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned as _;
    use core::net::SocketAddr;

    use time::OffsetDateTime;

    use super::DeviceSnapshot;
    use super::DeviceStatus;
    use crate::api::system::Identity;
    use crate::api::system::Routerboard;

    #[test]
    fn snapshot_prefers_serial_for_stable_key() {
        let snapshot = DeviceSnapshot {
            target_address: SocketAddr::from(([10, 0, 0, 1], 8728)),
            collected_at: OffsetDateTime::UNIX_EPOCH,
            status: DeviceStatus::Reachable,
            identity: Identity {
                name: Some("core".to_owned()),
            },
            routerboard: Routerboard {
                serial_number: Some("abc123".to_owned()),
                ..Routerboard::default()
            },
            ..DeviceSnapshot::default()
        };

        assert_eq!(snapshot.stable_key().as_str(), "abc123");
    }
}
