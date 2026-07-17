//! Device identity and raw/typed `RouterOS` endpoint snapshots.

use alloc::borrow::ToOwned as _;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;
use core::ops::DerefMut;
use core::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

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

/// `RouterBOARD` serial number used as the durable device identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct DeviceSerial(String);

impl DeviceSerial {
    /// Return the serial-number string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the serial and return its owned representation.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for DeviceSerial {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for DeviceSerial {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Display for DeviceSerial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DeviceSerial {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        crate::parse_non_empty(value).map(Self)
    }
}

impl TryFrom<String> for DeviceSerial {
    type Error = ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<'de> Deserialize<'de> for DeviceSerial {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Stable topology identity, including inferred and non-serial nodes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TopologyNodeKey(String);

impl TopologyNodeKey {
    /// Return the node-key string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TopologyNodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for TopologyNodeKey {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        crate::parse_non_empty(value).map(Self)
    }
}

impl From<String> for TopologyNodeKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl<'de> Deserialize<'de> for TopologyNodeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl From<DeviceSerial> for TopologyNodeKey {
    fn from(serial: DeviceSerial) -> Self {
        Self(serial.0)
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

/// Physical device classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// Router appliance.
    Router,
    /// Ethernet switch appliance.
    Switch,
    /// Wireless, cellular, `IoT`, or 60 GHz radio appliance.
    Radio,
}

impl DeviceKind {
    /// Return the uppercase label used by fleet interfaces.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Router => "ROUTER",
            Self::Switch => "SWITCH",
            Self::Radio => "RADIO",
        }
    }
}

impl fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Router => "router",
            Self::Switch => "switch",
            Self::Radio => "radio",
        })
    }
}

impl FromStr for DeviceKind {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "router" => Ok(Self::Router),
            "switch" => Ok(Self::Switch),
            "radio" => Ok(Self::Radio),
            _ => Err(ParseError::DeviceKind),
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

impl DeviceRole {
    /// Return the physical kind implied by this operator-assigned role.
    #[must_use]
    pub const fn kind(self) -> Option<DeviceKind> {
        match self {
            Self::Unknown => None,
            Self::BgpRouter | Self::CoreRouter | Self::CustomerRouter => Some(DeviceKind::Router),
            Self::Switch => Some(DeviceKind::Switch),
            Self::Radio => Some(DeviceKind::Radio),
        }
    }
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

/// Data and optional collection failure for one `RouterOS` endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointSnapshot<T> {
    /// Successfully decoded endpoint data, or its empty/default representation.
    pub data: T,
    /// Endpoint-local collection failure, when collection was unsuccessful.
    pub error: Option<EndpointError>,
}

impl<T> EndpointSnapshot<T> {
    /// Construct a successfully collected endpoint value.
    pub const fn success(data: T) -> Self {
        Self { data, error: None }
    }
}

impl<T> From<T> for EndpointSnapshot<T> {
    fn from(data: T) -> Self {
        Self::success(data)
    }
}

impl<T> Deref for EndpointSnapshot<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for EndpointSnapshot<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: Default> Default for EndpointSnapshot<T> {
    fn default() -> Self {
        Self::success(T::default())
    }
}

/// Serializable endpoint-local collection failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointError {
    /// Broad failure category suitable for UI and metrics labels.
    pub kind: EndpointErrorKind,
    /// Exact `RouterOS` command that failed.
    pub command: String,
    /// Safe diagnostic message.
    pub message: String,
}

/// Category of an endpoint-local collection failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointErrorKind {
    /// The endpoint is unavailable on this `RouterOS` release or platform.
    Unsupported,
    /// The authenticated API user lacks permission for the endpoint.
    PermissionDenied,
    /// `RouterOS` returned a command trap.
    RouterOsTrap,
    /// The endpoint command exceeded its deadline.
    Timeout,
    /// Returned rows could not be decoded.
    Decode,
    /// The underlying connection failed during the command.
    Transport,
}

/// Endpoint snapshots collected from the `/system` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// `/system/identity/print` row.
    pub identity: EndpointSnapshot<Identity>,
    /// `/system/resource/print` row.
    pub resource: EndpointSnapshot<Resource>,
    /// `/system/routerboard/print` row.
    pub routerboard: EndpointSnapshot<Routerboard>,
    /// `/system/clock/print` row.
    pub clock: EndpointSnapshot<Clock>,
    /// `/system/package/print` rows.
    pub packages: EndpointSnapshot<Vec<Package>>,
    /// `/system/package/update/print` rows.
    pub package_updates: EndpointSnapshot<Vec<PackageUpdate>>,
    /// `/system/health/print` rows.
    pub health: EndpointSnapshot<Vec<Health>>,
    /// `/system/resource/cpu/print` rows.
    pub resource_cpus: EndpointSnapshot<Vec<ResourceCpu>>,
    /// `/system/resource/hardware/print` rows.
    pub resource_hardware: EndpointSnapshot<Vec<ResourceHardware>>,
    /// `/system/resource/irq/print` rows.
    pub resource_irqs: EndpointSnapshot<Vec<ResourceIrq>>,
    /// `/system/resource/usb/settings/print` rows.
    pub resource_usb_settings: EndpointSnapshot<Vec<ResourceUsbSettings>>,
    /// `/system/routerboard/settings/print` rows.
    pub routerboard_settings: EndpointSnapshot<Vec<RouterboardSettings>>,
    /// `/system/routerboard/reset-button/print` rows.
    pub routerboard_reset_buttons: EndpointSnapshot<Vec<RouterboardResetButton>>,
    /// `/system/device-mode/print` rows.
    pub device_modes: EndpointSnapshot<Vec<DeviceMode>>,
    /// `/system/history/print` rows.
    pub history_entries: EndpointSnapshot<Vec<HistoryEntry>>,
    /// `/system/leds/print` rows.
    pub leds: EndpointSnapshot<Vec<Led>>,
    /// `/system/license/print` rows.
    pub licenses: EndpointSnapshot<Vec<License>>,
    /// `/log/print` rows.
    pub log_entries: EndpointSnapshot<Vec<LogEntry>>,
    /// `/system/logging/print` rows.
    pub logging_rules: EndpointSnapshot<Vec<LoggingRule>>,
    /// `/system/logging/action/print` rows.
    pub logging_actions: EndpointSnapshot<Vec<LoggingAction>>,
    /// `/system/note/print` rows.
    pub notes: EndpointSnapshot<Vec<Note>>,
    /// `/system/ntp/client/print` rows.
    pub ntp_clients: EndpointSnapshot<Vec<NtpClient>>,
    /// `/system/ntp/server/print` rows.
    pub ntp_servers: EndpointSnapshot<Vec<NtpServer>>,
    /// `/system/script/print` rows.
    pub scripts: EndpointSnapshot<Vec<Script>>,
    /// `/system/script/job/print` rows.
    pub script_jobs: EndpointSnapshot<Vec<ScriptJob>>,
    /// `/system/scheduler/print` rows.
    pub schedulers: EndpointSnapshot<Vec<Scheduler>>,
    /// `/system/upgrade/mirror/print` rows.
    pub upgrade_mirrors: EndpointSnapshot<Vec<UpgradeMirror>>,
    /// `/system/watchdog/print` rows.
    pub watchdogs: EndpointSnapshot<Vec<Watchdog>>,
}

/// Endpoint snapshots collected from the `/interface` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InterfaceSnapshot {
    /// `/interface/print` rows.
    pub interfaces: EndpointSnapshot<Vec<Interface>>,
    /// `/interface/ethernet/print` rows.
    pub ethernet_interfaces: EndpointSnapshot<Vec<EthernetInterface>>,
    /// `/interface/bridge/print` rows.
    pub bridges: EndpointSnapshot<Vec<Bridge>>,
    /// `/interface/bridge/host/print` rows.
    pub bridge_hosts: EndpointSnapshot<Vec<BridgeHost>>,
    /// `/interface/bridge/port/print` rows.
    pub bridge_ports: EndpointSnapshot<Vec<BridgePort>>,
    /// `/interface/bridge/settings/print` rows.
    pub bridge_settings: EndpointSnapshot<Vec<BridgeSettings>>,
    /// `/interface/bridge/vlan/print` rows.
    pub bridge_vlans: EndpointSnapshot<Vec<BridgeVlan>>,
    /// `/interface/detect-internet/print` rows.
    pub detect_internet: EndpointSnapshot<Vec<DetectInternet>>,
    /// `/interface/ethernet/switch/print` rows.
    pub ethernet_switches: EndpointSnapshot<Vec<EthernetSwitch>>,
    /// `/interface/ethernet/switch/port/print` rows.
    pub ethernet_switch_ports: EndpointSnapshot<Vec<EthernetSwitchPort>>,
    /// `/interface/ethernet/switch/port-isolation/print` rows.
    pub ethernet_switch_port_isolations: EndpointSnapshot<Vec<EthernetSwitchPortIsolation>>,
    /// `/interface/list/print` rows.
    pub interface_lists: EndpointSnapshot<Vec<InterfaceList>>,
    /// `/interface/list/member/print` rows.
    pub interface_list_members: EndpointSnapshot<Vec<InterfaceListMember>>,
    /// `/interface/lte/apn/print` rows.
    pub lte_apns: EndpointSnapshot<Vec<LteApn>>,
    /// `/interface/vlan/print` rows.
    pub vlan_interfaces: EndpointSnapshot<Vec<VlanInterface>>,
    /// `/interface/wireguard/print` rows.
    pub wireguard_interfaces: EndpointSnapshot<Vec<WireGuardInterface>>,
    /// `/interface/wireguard/peers/print` rows.
    pub wireguard_peers: EndpointSnapshot<Vec<WireGuardPeer>>,
    /// `/interface/wireless/security-profiles/print` rows.
    pub wireless_security_profiles: EndpointSnapshot<Vec<WirelessSecurityProfile>>,
}

/// Endpoint snapshots collected from the `/ip` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IpSnapshot {
    /// `/ip/neighbor/print` rows.
    pub neighbors: EndpointSnapshot<Vec<Neighbor>>,
    /// `/ip/address/print` rows.
    pub addresses: EndpointSnapshot<Vec<Address>>,
    /// `/ip/arp/print` rows.
    pub arp_entries: EndpointSnapshot<Vec<ArpEntry>>,
    /// `/ip/dhcp-client/print` rows.
    pub dhcp_clients: EndpointSnapshot<Vec<DhcpClient>>,
    /// `/ip/dhcp-server/print` rows.
    pub dhcp_servers: EndpointSnapshot<Vec<DhcpServer>>,
    /// `/ip/dhcp-server/network/print` rows.
    pub dhcp_server_networks: EndpointSnapshot<Vec<DhcpServerNetwork>>,
    /// `/ip/dhcp-server/lease/print` rows.
    pub dhcp_leases: EndpointSnapshot<Vec<DhcpLease>>,
    /// `/ip/dns/print` rows.
    pub dns: EndpointSnapshot<Vec<Dns>>,
    /// `/ip/dns/cache/print` rows.
    pub dns_cache_entries: EndpointSnapshot<Vec<DnsCacheEntry>>,
    /// `/ip/route/print` rows.
    pub routes: EndpointSnapshot<Vec<Route>>,
    /// `/ip/firewall/filter/print` rows.
    pub firewall_filter_rules: EndpointSnapshot<Vec<FirewallRule>>,
    /// `/ip/firewall/nat/print` rows.
    pub firewall_nat_rules: EndpointSnapshot<Vec<FirewallRule>>,
    /// `/ip/firewall/address-list/print` rows.
    pub firewall_address_list_entries: EndpointSnapshot<Vec<FirewallAddressListEntry>>,
    /// `/ip/firewall/connection/print` rows.
    pub firewall_connections: EndpointSnapshot<Vec<FirewallConnection>>,
    /// `/ip/firewall/connection/tracking/print` rows.
    pub firewall_connection_tracking: EndpointSnapshot<Vec<FirewallConnectionTracking>>,
    /// `/ip/firewall/mangle/print` rows.
    pub firewall_mangle_rules: EndpointSnapshot<Vec<FirewallRule>>,
    /// `/ip/firewall/raw/print` rows.
    pub firewall_raw_rules: EndpointSnapshot<Vec<FirewallRule>>,
    /// `/ip/firewall/service-port/print` rows.
    pub firewall_service_ports: EndpointSnapshot<Vec<FirewallServicePort>>,
    /// `/ip/hotspot/profile/print` rows.
    pub hotspot_profiles: EndpointSnapshot<Vec<HotspotProfile>>,
    /// `/ip/hotspot/user/print` rows.
    pub hotspot_users: EndpointSnapshot<Vec<HotspotUser>>,
    /// `/ip/cloud/print` rows.
    pub ip_cloud: EndpointSnapshot<Vec<IpCloud>>,
    /// `/ip/pool/print` rows.
    pub ip_pools: EndpointSnapshot<Vec<IpPool>>,
    /// `/ip/pool/used/print` rows.
    pub ip_pool_used: EndpointSnapshot<Vec<IpPoolUsed>>,
    /// `/ip/proxy/print` rows.
    pub ip_proxy: EndpointSnapshot<Vec<IpProxy>>,
    /// `/ip/service/print` rows.
    pub ip_services: EndpointSnapshot<Vec<IpService>>,
    /// `/ip/settings/print` rows.
    pub ip_settings: EndpointSnapshot<Vec<IpSettings>>,
    /// `/ip/ipsec/policy/print` rows.
    pub ipsec_policies: EndpointSnapshot<Vec<IpsecPolicy>>,
    /// `/ip/ipsec/profile/print` rows.
    pub ipsec_profiles: EndpointSnapshot<Vec<IpsecProfile>>,
    /// `/ip/ipsec/proposal/print` rows.
    pub ipsec_proposals: EndpointSnapshot<Vec<IpsecProposal>>,
    /// `/ip/ipsec/statistics/print` rows.
    pub ipsec_statistics: EndpointSnapshot<Vec<IpsecStatistics>>,
    /// `/ip/nat-pmp/print` rows.
    pub nat_pmp: EndpointSnapshot<Vec<NatPmp>>,
    /// `/ip/neighbor/discovery-settings/print` rows.
    pub neighbor_discovery_settings: EndpointSnapshot<Vec<NeighborDiscoverySettings>>,
    /// `/ip/smb/print` rows.
    pub smb: EndpointSnapshot<Vec<Smb>>,
    /// `/ip/smb/shares/print` rows.
    pub smb_shares: EndpointSnapshot<Vec<SmbShare>>,
    /// `/ip/socks/print` rows.
    pub socks: EndpointSnapshot<Vec<Socks>>,
    /// `/ip/ssh/print` rows.
    pub ssh: EndpointSnapshot<Vec<Ssh>>,
    /// `/ip/traffic-flow/print` rows.
    pub traffic_flow: EndpointSnapshot<Vec<TrafficFlow>>,
    /// `/ip/upnp/print` rows.
    pub upnp: EndpointSnapshot<Vec<Upnp>>,
    /// `/ip/vrf/print` rows.
    pub vrfs: EndpointSnapshot<Vec<Vrf>>,
}

/// Endpoint snapshots collected from the `/ipv6` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ipv6Snapshot {
    /// `/ipv6/address/print` rows.
    pub ipv6_addresses: EndpointSnapshot<Vec<Ipv6Address>>,
    /// `/ipv6/neighbor/print` rows.
    pub ipv6_neighbors: EndpointSnapshot<Vec<Ipv6Neighbor>>,
    /// `/ipv6/nd/print` rows.
    pub ipv6_neighbor_discovery: EndpointSnapshot<Vec<Ipv6NeighborDiscovery>>,
    /// `/ipv6/route/print` rows.
    pub ipv6_routes: EndpointSnapshot<Vec<Ipv6Route>>,
    /// `/ipv6/settings/print` rows.
    pub ipv6_settings: EndpointSnapshot<Vec<Ipv6Settings>>,
}

/// Endpoint snapshots collected from the `/certificate` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CertificateSnapshot {
    /// `/certificate/print` rows.
    pub certificates: EndpointSnapshot<Vec<Certificate>>,
    /// `/certificate/settings/print` rows.
    pub certificate_settings: EndpointSnapshot<Vec<CertificateSettings>>,
}

/// Endpoint snapshots collected from the `/console` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConsoleSnapshot {
    /// `/console/settings/print` rows.
    pub console_settings: EndpointSnapshot<Vec<ConsoleSettings>>,
}

/// Endpoint snapshots collected from the `/disk` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskSnapshot {
    /// `/disk/print` rows.
    pub disks: EndpointSnapshot<Vec<Disk>>,
    /// `/disk/settings/print` rows.
    pub disk_settings: EndpointSnapshot<Vec<DiskSettings>>,
}

/// Endpoint snapshots collected from the `/file` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileSnapshot {
    /// `/file/print` rows.
    pub files: EndpointSnapshot<Vec<File>>,
}

/// Endpoint snapshots collected from the `/partitions` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartitionsSnapshot {
    /// `/partitions/print` rows.
    pub partitions: EndpointSnapshot<Vec<Partition>>,
}

/// Endpoint snapshots collected from the `/caps-man` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CapsManSnapshot {
    /// `/caps-man/aaa/print` rows.
    pub caps_man_aaa: EndpointSnapshot<Vec<CapsManAaa>>,
    /// `/caps-man/manager/print` rows.
    pub caps_man_managers: EndpointSnapshot<Vec<CapsManManager>>,
    /// `/caps-man/manager/interface/print` rows.
    pub caps_man_manager_interfaces: EndpointSnapshot<Vec<CapsManManagerInterface>>,
}

/// Endpoint snapshots collected from the `/mpls` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MplsSnapshot {
    /// `/mpls/settings/print` rows.
    pub mpls_settings: EndpointSnapshot<Vec<MplsSettings>>,
}

/// Endpoint snapshots collected from the `/ppp` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PppSnapshot {
    /// `/ppp/aaa/print` rows.
    pub ppp_aaa: EndpointSnapshot<Vec<PppAaa>>,
    /// `/ppp/profile/print` rows.
    pub ppp_profiles: EndpointSnapshot<Vec<PppProfile>>,
}

/// Endpoint snapshots collected from the `/radius` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RadiusSnapshot {
    /// `/radius/incoming/print` rows.
    pub radius_incoming: EndpointSnapshot<Vec<RadiusIncoming>>,
}

/// Endpoint snapshots collected from the `/queue` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueueSnapshot {
    /// `/queue/interface/print` rows.
    pub queue_interfaces: EndpointSnapshot<Vec<QueueInterface>>,
    /// `/queue/type/print` rows.
    pub queue_types: EndpointSnapshot<Vec<QueueType>>,
}

/// Endpoint snapshots collected from the `/snmp` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SnmpSnapshot {
    /// `/snmp/print` rows.
    pub snmp: EndpointSnapshot<Vec<Snmp>>,
    /// `/snmp/community/print` rows.
    pub snmp_communities: EndpointSnapshot<Vec<SnmpCommunity>>,
}

/// Endpoint snapshots collected from the `/tool` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolSnapshot {
    /// `/tool/bandwidth-server/print` rows.
    pub bandwidth_servers: EndpointSnapshot<Vec<BandwidthServer>>,
    /// `/tool/e-mail/print` rows.
    pub emails: EndpointSnapshot<Vec<Email>>,
    /// `/tool/graphing/print` rows.
    pub graphing: EndpointSnapshot<Vec<Graphing>>,
    /// `/tool/mac-server/ping/print` rows.
    pub mac_server_pings: EndpointSnapshot<Vec<MacServerPing>>,
    /// `/tool/romon/print` rows.
    pub romon: EndpointSnapshot<Vec<Romon>>,
    /// `/tool/romon/port/print` rows.
    pub romon_ports: EndpointSnapshot<Vec<RomonPort>>,
    /// `/tool/sms/print` rows.
    pub sms: EndpointSnapshot<Vec<Sms>>,
    /// `/tool/sniffer/print` rows.
    pub sniffers: EndpointSnapshot<Vec<Sniffer>>,
    /// `/tool/traffic-generator/print` rows.
    pub traffic_generators: EndpointSnapshot<Vec<TrafficGenerator>>,
    /// `/tool/traffic-generator/stats/latency-distribution/print` rows.
    pub traffic_generator_latency_distributions: EndpointSnapshot<Vec<TrafficGeneratorLatencyDistribution>>,
}

/// Endpoint snapshots collected from the `/routing` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RoutingSnapshot {
    /// `/routing/bgp/session/print` rows.
    pub bgp_sessions: EndpointSnapshot<Vec<BgpSession>>,
    /// `/routing/bgp/connection/print` rows.
    pub bgp_connections: EndpointSnapshot<Vec<BgpConnection>>,
    /// `RouterOS` v6 `/routing/bgp/peer/print` rows.
    pub bgp_peers: EndpointSnapshot<Vec<BgpPeer>>,
    /// `/routing/bgp/template/print` rows.
    pub bgp_templates: EndpointSnapshot<Vec<BgpTemplate>>,
    /// `/routing/igmp-proxy/print` rows.
    pub igmp_proxy: EndpointSnapshot<Vec<IgmpProxy>>,
    /// `/routing/id/print` rows.
    pub routing_ids: EndpointSnapshot<Vec<RoutingId>>,
    /// `/routing/nexthop/print` rows.
    pub routing_nexthops: EndpointSnapshot<Vec<RoutingNexthop>>,
    /// `/routing/route/print` rows.
    pub routing_routes: EndpointSnapshot<Vec<RoutingRoute>>,
    /// `/routing/settings/print` rows.
    pub routing_settings: EndpointSnapshot<Vec<RoutingSettings>>,
    /// `/routing/stats/memory/print` rows.
    pub routing_stats_memory: EndpointSnapshot<Vec<RoutingStatsMemory>>,
    /// `/routing/stats/origin/print` rows.
    pub routing_stats_origin: EndpointSnapshot<Vec<RoutingStatsOrigin>>,
    /// `/routing/stats/process/print` rows.
    pub routing_stats_processes: EndpointSnapshot<Vec<RoutingStatsProcess>>,
    /// `/routing/stats/step/print` rows.
    pub routing_stats_steps: EndpointSnapshot<Vec<RoutingStatsStep>>,
    /// `/routing/table/print` rows.
    pub routing_tables: EndpointSnapshot<Vec<RoutingTable>>,
}

/// Endpoint snapshots collected from the `/user` section.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserSnapshot {
    /// `/user/active/print` rows.
    pub active_users: EndpointSnapshot<Vec<ActiveUser>>,
    /// `/user/ssh-keys/print` rows.
    pub ssh_keys: EndpointSnapshot<Vec<SshKey>>,
    /// `/user/print` rows.
    pub users: EndpointSnapshot<Vec<User>>,
    /// `/user/aaa/print` rows.
    pub user_aaa: EndpointSnapshot<Vec<UserAaa>>,
    /// `/user/group/print` rows.
    pub user_groups: EndpointSnapshot<Vec<UserGroup>>,
    /// `/user/settings/print` rows.
    pub user_settings: EndpointSnapshot<Vec<UserSettings>>,
}

/// Typed and raw endpoint rows read from one `RouterOS` device.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RouterOsSnapshot {
    /// Collected `/system` endpoints.
    pub system: SystemSnapshot,
    /// Collected `/interface` endpoints.
    pub interface: InterfaceSnapshot,
    /// Collected `/ip` endpoints.
    pub ip: IpSnapshot,
    /// Collected `/ipv6` endpoints.
    pub ipv6: Ipv6Snapshot,
    /// Collected `/certificate` endpoints.
    pub certificate: CertificateSnapshot,
    /// Collected `/console` endpoints.
    pub console: ConsoleSnapshot,
    /// Collected `/disk` endpoints.
    pub disk: DiskSnapshot,
    /// Collected `/file` endpoints.
    pub file: FileSnapshot,
    /// Collected `/partitions` endpoints.
    pub partitions: PartitionsSnapshot,
    /// Collected `/caps-man` endpoints.
    pub caps_man: CapsManSnapshot,
    /// Collected `/mpls` endpoints.
    pub mpls: MplsSnapshot,
    /// Collected `/ppp` endpoints.
    pub ppp: PppSnapshot,
    /// Collected `/radius` endpoints.
    pub radius: RadiusSnapshot,
    /// Collected `/queue` endpoints.
    pub queue: QueueSnapshot,
    /// Collected `/snmp` endpoints.
    pub snmp: SnmpSnapshot,
    /// Collected `/tool` endpoints.
    pub tool: ToolSnapshot,
    /// Collected `/routing` endpoints.
    pub routing: RoutingSnapshot,
    /// Collected `/user` endpoints.
    pub user: UserSnapshot,
    /// Raw `RouterOS` rows by endpoint name.
    pub raw: BTreeMap<String, Vec<Row>>,
}

impl RouterOsSnapshot {
    /// Return the `RouterBOARD` serial exposed by `RouterOS`.
    #[must_use]
    pub fn device_serial(&self) -> Option<DeviceSerial> {
        self.system
            .routerboard
            .data
            .serial_number
            .as_deref()
            .and_then(|value| value.parse().ok())
    }

    /// Return the strongest topology identity exposed by `RouterOS`.
    #[must_use]
    pub fn topology_node_key(&self) -> Option<TopologyNodeKey> {
        self.device_serial().map(Into::into).or_else(|| {
            self.system
                .identity
                .data
                .name
                .as_deref()
                .map(|value| value.to_owned().into())
        })
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

#[cfg(test)]
mod tests {
    use alloc::borrow::ToOwned as _;

    use super::RouterOsSnapshot;
    use super::SystemSnapshot;
    use crate::api::system::Identity;
    use crate::api::system::Routerboard;

    #[test]
    fn snapshot_exposes_serial_and_prefers_it_for_topology() {
        let snapshot = RouterOsSnapshot {
            system: SystemSnapshot {
                identity: Identity {
                    name: Some("core".to_owned()),
                }
                .into(),
                routerboard: Routerboard {
                    serial_number: Some("abc123".to_owned()),
                    ..Routerboard::default()
                }
                .into(),
                ..SystemSnapshot::default()
            },
            ..RouterOsSnapshot::default()
        };

        assert_eq!(
            snapshot.device_serial().as_ref().map(super::DeviceSerial::as_str),
            Some("abc123")
        );
        assert_eq!(
            snapshot
                .topology_node_key()
                .as_ref()
                .map(super::TopologyNodeKey::as_str),
            Some("abc123")
        );
    }
}
