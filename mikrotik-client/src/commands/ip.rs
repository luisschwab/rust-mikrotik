//! `RouterOS` IP and IPv6 print command paths.

use core::fmt;

/// `RouterOS` print command `/ip/address/print`.
const IP_ADDRESS_PRINT: &str = "/ip/address/print";

/// `RouterOS` print command `/ip/arp/print`.
const IP_ARP_ENTRY_PRINT: &str = "/ip/arp/print";

/// `RouterOS` print command `/ip/dhcp-client/print`.
const IP_DHCP_CLIENT_PRINT: &str = "/ip/dhcp-client/print";

/// `RouterOS` print command `/ip/dhcp-server/lease/print`.
const IP_DHCP_LEASE_PRINT: &str = "/ip/dhcp-server/lease/print";

/// `RouterOS` print command `/ip/dhcp-server/print`.
const IP_DHCP_SERVER_PRINT: &str = "/ip/dhcp-server/print";

/// `RouterOS` print command `/ip/dhcp-server/network/print`.
const IP_DHCP_SERVER_NETWORK_PRINT: &str = "/ip/dhcp-server/network/print";

/// `RouterOS` print command `/ip/dns/print`.
const IP_DNS_PRINT: &str = "/ip/dns/print";

/// `RouterOS` print command `/ip/dns/cache/print`.
const IP_DNS_CACHE_ENTRY_PRINT: &str = "/ip/dns/cache/print";

/// `RouterOS` print command `/ip/firewall/address-list/print`.
const IP_FIREWALL_ADDRESS_LIST_ENTRY_PRINT: &str = "/ip/firewall/address-list/print";

/// `RouterOS` print command `/ip/firewall/connection/print`.
const IP_FIREWALL_CONNECTION_PRINT: &str = "/ip/firewall/connection/print";

/// `RouterOS` print command `/ip/firewall/connection/tracking/print`.
const IP_FIREWALL_CONNECTION_TRACKING_PRINT: &str = "/ip/firewall/connection/tracking/print";

/// `RouterOS` print command `/ip/firewall/filter/print`.
const IP_FIREWALL_RULE_FILTER_PRINT: &str = "/ip/firewall/filter/print";

/// `RouterOS` print command `/ip/firewall/mangle/print`.
const IP_FIREWALL_RULE_MANGLE_PRINT: &str = "/ip/firewall/mangle/print";

/// `RouterOS` print command `/ip/firewall/nat/print`.
const IP_FIREWALL_RULE_NAT_PRINT: &str = "/ip/firewall/nat/print";

/// `RouterOS` print command `/ip/firewall/raw/print`.
const IP_FIREWALL_RULE_RAW_PRINT: &str = "/ip/firewall/raw/print";

/// `RouterOS` print command `/ip/firewall/service-port/print`.
const IP_FIREWALL_SERVICE_PORT_PRINT: &str = "/ip/firewall/service-port/print";

/// `RouterOS` print command `/ip/hotspot/profile/print`.
const IP_HOTSPOT_PROFILE_PRINT: &str = "/ip/hotspot/profile/print";

/// `RouterOS` print command `/ip/hotspot/user/print`.
const IP_HOTSPOT_USER_PRINT: &str = "/ip/hotspot/user/print";

/// `RouterOS` print command `/ip/cloud/print`.
const IP_IP_CLOUD_PRINT: &str = "/ip/cloud/print";

/// `RouterOS` print command `/ip/pool/print`.
const IP_IP_POOL_PRINT: &str = "/ip/pool/print";

/// `RouterOS` print command `/ip/pool/used/print`.
const IP_IP_POOL_USED_PRINT: &str = "/ip/pool/used/print";

/// `RouterOS` print command `/ip/proxy/print`.
const IP_IP_PROXY_PRINT: &str = "/ip/proxy/print";

/// `RouterOS` print command `/ip/service/print`.
const IP_IP_SERVICE_PRINT: &str = "/ip/service/print";

/// `RouterOS` print command `/ip/settings/print`.
const IP_IP_SETTINGS_PRINT: &str = "/ip/settings/print";

/// `RouterOS` print command `/ip/ipsec/policy/print`.
const IP_IPSEC_POLICY_PRINT: &str = "/ip/ipsec/policy/print";

/// `RouterOS` print command `/ip/ipsec/profile/print`.
const IP_IPSEC_PROFILE_PRINT: &str = "/ip/ipsec/profile/print";

/// `RouterOS` print command `/ip/ipsec/proposal/print`.
const IP_IPSEC_PROPOSAL_PRINT: &str = "/ip/ipsec/proposal/print";

/// `RouterOS` print command `/ip/ipsec/statistics/print`.
const IP_IPSEC_STATISTICS_PRINT: &str = "/ip/ipsec/statistics/print";

/// `RouterOS` print command `/ipv6/address/print`.
const IP_IPV6_ADDRESS_PRINT: &str = "/ipv6/address/print";

/// `RouterOS` print command `/ipv6/neighbor/print`.
const IP_IPV6_NEIGHBOR_PRINT: &str = "/ipv6/neighbor/print";

/// `RouterOS` print command `/ipv6/nd/print`.
const IP_IPV6_NEIGHBOR_DISCOVERY_PRINT: &str = "/ipv6/nd/print";

/// `RouterOS` print command `/ipv6/route/print`.
const IP_IPV6_ROUTE_PRINT: &str = "/ipv6/route/print";

/// `RouterOS` print command `/ipv6/settings/print`.
const IP_IPV6_SETTINGS_PRINT: &str = "/ipv6/settings/print";

/// `RouterOS` print command `/ip/nat-pmp/print`.
const IP_NAT_PMP_PRINT: &str = "/ip/nat-pmp/print";

/// `RouterOS` print command `/ip/neighbor/print`.
const IP_NEIGHBOR_PRINT: &str = "/ip/neighbor/print";

/// `RouterOS` print command `/ip/neighbor/discovery-settings/print`.
const IP_NEIGHBOR_DISCOVERY_SETTINGS_PRINT: &str = "/ip/neighbor/discovery-settings/print";

/// `RouterOS` print command `/ip/route/print`.
const IP_ROUTE_PRINT: &str = "/ip/route/print";

/// `RouterOS` print command `/ip/smb/print`.
const IP_SMB_PRINT: &str = "/ip/smb/print";

/// `RouterOS` print command `/ip/smb/shares/print`.
const IP_SMB_SHARE_PRINT: &str = "/ip/smb/shares/print";

/// `RouterOS` print command `/ip/socks/print`.
const IP_SOCKS_PRINT: &str = "/ip/socks/print";

/// `RouterOS` print command `/ip/ssh/print`.
const IP_SSH_PRINT: &str = "/ip/ssh/print";

/// `RouterOS` print command `/ip/traffic-flow/print`.
const IP_TRAFFIC_FLOW_PRINT: &str = "/ip/traffic-flow/print";

/// `RouterOS` print command `/ip/upnp/print`.
const IP_UPNP_PRINT: &str = "/ip/upnp/print";

/// `RouterOS` print command `/ip/vrf/print`.
const IP_VRF_PRINT: &str = "/ip/vrf/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ip {
    /// `RouterOS` print command.
    Address,
    /// `RouterOS` print command.
    ArpEntry,
    /// `RouterOS` print command.
    DhcpClient,
    /// `RouterOS` print command.
    DhcpLease,
    /// `RouterOS` print command.
    DhcpServer,
    /// `RouterOS` print command.
    DhcpServerNetwork,
    /// `RouterOS` print command.
    Dns,
    /// `RouterOS` print command.
    DnsCacheEntry,
    /// `RouterOS` print command.
    FirewallAddressListEntry,
    /// `RouterOS` print command.
    FirewallConnection,
    /// `RouterOS` print command.
    FirewallConnectionTracking,
    /// `RouterOS` print command.
    FirewallRuleFilter,
    /// `RouterOS` print command.
    FirewallRuleMangle,
    /// `RouterOS` print command.
    FirewallRuleNat,
    /// `RouterOS` print command.
    FirewallRuleRaw,
    /// `RouterOS` print command.
    FirewallServicePort,
    /// `RouterOS` print command.
    HotspotProfile,
    /// `RouterOS` print command.
    HotspotUser,
    /// `RouterOS` print command.
    IpCloud,
    /// `RouterOS` print command.
    IpPool,
    /// `RouterOS` print command.
    IpPoolUsed,
    /// `RouterOS` print command.
    IpProxy,
    /// `RouterOS` print command.
    IpService,
    /// `RouterOS` print command.
    IpSettings,
    /// `RouterOS` print command.
    IpsecPolicy,
    /// `RouterOS` print command.
    IpsecProfile,
    /// `RouterOS` print command.
    IpsecProposal,
    /// `RouterOS` print command.
    IpsecStatistics,
    /// `RouterOS` print command.
    Ipv6Address,
    /// `RouterOS` print command.
    Ipv6Neighbor,
    /// `RouterOS` print command.
    Ipv6NeighborDiscovery,
    /// `RouterOS` print command.
    Ipv6Route,
    /// `RouterOS` print command.
    Ipv6Settings,
    /// `RouterOS` print command.
    NatPmp,
    /// `RouterOS` print command.
    Neighbor,
    /// `RouterOS` print command.
    NeighborDiscoverySettings,
    /// `RouterOS` print command.
    Route,
    /// `RouterOS` print command.
    Smb,
    /// `RouterOS` print command.
    SmbShare,
    /// `RouterOS` print command.
    Socks,
    /// `RouterOS` print command.
    Ssh,
    /// `RouterOS` print command.
    TrafficFlow,
    /// `RouterOS` print command.
    Upnp,
    /// `RouterOS` print command.
    Vrf,
}

impl Ip {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::Address,
        Self::ArpEntry,
        Self::DhcpClient,
        Self::DhcpLease,
        Self::DhcpServer,
        Self::DhcpServerNetwork,
        Self::Dns,
        Self::DnsCacheEntry,
        Self::FirewallAddressListEntry,
        Self::FirewallConnection,
        Self::FirewallConnectionTracking,
        Self::FirewallRuleFilter,
        Self::FirewallRuleMangle,
        Self::FirewallRuleNat,
        Self::FirewallRuleRaw,
        Self::FirewallServicePort,
        Self::HotspotProfile,
        Self::HotspotUser,
        Self::IpCloud,
        Self::IpPool,
        Self::IpPoolUsed,
        Self::IpProxy,
        Self::IpService,
        Self::IpSettings,
        Self::IpsecPolicy,
        Self::IpsecProfile,
        Self::IpsecProposal,
        Self::IpsecStatistics,
        Self::Ipv6Address,
        Self::Ipv6Neighbor,
        Self::Ipv6NeighborDiscovery,
        Self::Ipv6Route,
        Self::Ipv6Settings,
        Self::NatPmp,
        Self::Neighbor,
        Self::NeighborDiscoverySettings,
        Self::Route,
        Self::Smb,
        Self::SmbShare,
        Self::Socks,
        Self::Ssh,
        Self::TrafficFlow,
        Self::Upnp,
        Self::Vrf,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::Address => IP_ADDRESS_PRINT,
            Self::ArpEntry => IP_ARP_ENTRY_PRINT,
            Self::DhcpClient => IP_DHCP_CLIENT_PRINT,
            Self::DhcpLease => IP_DHCP_LEASE_PRINT,
            Self::DhcpServer => IP_DHCP_SERVER_PRINT,
            Self::DhcpServerNetwork => IP_DHCP_SERVER_NETWORK_PRINT,
            Self::Dns => IP_DNS_PRINT,
            Self::DnsCacheEntry => IP_DNS_CACHE_ENTRY_PRINT,
            Self::FirewallAddressListEntry => IP_FIREWALL_ADDRESS_LIST_ENTRY_PRINT,
            Self::FirewallConnection => IP_FIREWALL_CONNECTION_PRINT,
            Self::FirewallConnectionTracking => IP_FIREWALL_CONNECTION_TRACKING_PRINT,
            Self::FirewallRuleFilter => IP_FIREWALL_RULE_FILTER_PRINT,
            Self::FirewallRuleMangle => IP_FIREWALL_RULE_MANGLE_PRINT,
            Self::FirewallRuleNat => IP_FIREWALL_RULE_NAT_PRINT,
            Self::FirewallRuleRaw => IP_FIREWALL_RULE_RAW_PRINT,
            Self::FirewallServicePort => IP_FIREWALL_SERVICE_PORT_PRINT,
            Self::HotspotProfile => IP_HOTSPOT_PROFILE_PRINT,
            Self::HotspotUser => IP_HOTSPOT_USER_PRINT,
            Self::IpCloud => IP_IP_CLOUD_PRINT,
            Self::IpPool => IP_IP_POOL_PRINT,
            Self::IpPoolUsed => IP_IP_POOL_USED_PRINT,
            Self::IpProxy => IP_IP_PROXY_PRINT,
            Self::IpService => IP_IP_SERVICE_PRINT,
            Self::IpSettings => IP_IP_SETTINGS_PRINT,
            Self::IpsecPolicy => IP_IPSEC_POLICY_PRINT,
            Self::IpsecProfile => IP_IPSEC_PROFILE_PRINT,
            Self::IpsecProposal => IP_IPSEC_PROPOSAL_PRINT,
            Self::IpsecStatistics => IP_IPSEC_STATISTICS_PRINT,
            Self::Ipv6Address => IP_IPV6_ADDRESS_PRINT,
            Self::Ipv6Neighbor => IP_IPV6_NEIGHBOR_PRINT,
            Self::Ipv6NeighborDiscovery => IP_IPV6_NEIGHBOR_DISCOVERY_PRINT,
            Self::Ipv6Route => IP_IPV6_ROUTE_PRINT,
            Self::Ipv6Settings => IP_IPV6_SETTINGS_PRINT,
            Self::NatPmp => IP_NAT_PMP_PRINT,
            Self::Neighbor => IP_NEIGHBOR_PRINT,
            Self::NeighborDiscoverySettings => IP_NEIGHBOR_DISCOVERY_SETTINGS_PRINT,
            Self::Route => IP_ROUTE_PRINT,
            Self::Smb => IP_SMB_PRINT,
            Self::SmbShare => IP_SMB_SHARE_PRINT,
            Self::Socks => IP_SOCKS_PRINT,
            Self::Ssh => IP_SSH_PRINT,
            Self::TrafficFlow => IP_TRAFFIC_FLOW_PRINT,
            Self::Upnp => IP_UPNP_PRINT,
            Self::Vrf => IP_VRF_PRINT,
        }
    }
}

impl fmt::Display for Ip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}
