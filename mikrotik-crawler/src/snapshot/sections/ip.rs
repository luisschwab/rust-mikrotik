//! `/ip` and `/ipv6` snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::ip::Ip;
use mikrotik_types::device::EndpointSnapshot;
use mikrotik_types::device::IpSnapshot;
use mikrotik_types::device::Ipv6Snapshot;

use super::EndpointCollector;
use crate::error::Result;

/// Collect required addressing and neighbor data plus optional `/ip` endpoints.
pub(super) async fn collect_ip(collector: &EndpointCollector<'_>) -> Result<IpSnapshot> {
    Ok(IpSnapshot {
        neighbors: collector.required_many(command(Ip::Neighbor)).await?,
        addresses: collector.required_many(command(Ip::Address)).await?,
        arp_entries: collector.optional_many(command(Ip::ArpEntry)).await,
        dhcp_clients: collector.optional_many(command(Ip::DhcpClient)).await,
        dhcp_servers: collector.optional_many(command(Ip::DhcpServer)).await,
        dhcp_server_networks: collector.optional_many(command(Ip::DhcpServerNetwork)).await,
        dhcp_leases: collector.optional_many(command(Ip::DhcpLease)).await,
        dns: collector.optional_many(command(Ip::Dns)).await,
        dns_cache_entries: collector.optional_many(command(Ip::DnsCacheEntry)).await,
        routes: collector.optional_many(command(Ip::Route)).await,
        firewall_filter_rules: collector.optional_many(command(Ip::FirewallRuleFilter)).await,
        firewall_nat_rules: collector.optional_many(command(Ip::FirewallRuleNat)).await,
        firewall_address_list_entries: collector.optional_many(command(Ip::FirewallAddressListEntry)).await,
        firewall_connections: collector.optional_many(command(Ip::FirewallConnection)).await,
        firewall_connection_tracking: collector.optional_many(command(Ip::FirewallConnectionTracking)).await,
        firewall_mangle_rules: collector.optional_many(command(Ip::FirewallRuleMangle)).await,
        firewall_raw_rules: collector.optional_many(command(Ip::FirewallRuleRaw)).await,
        firewall_service_ports: collector.optional_many(command(Ip::FirewallServicePort)).await,
        hotspot_profiles: collector.optional_many(command(Ip::HotspotProfile)).await,
        hotspot_users: collector.optional_many(command(Ip::HotspotUser)).await,
        ip_cloud: collector.optional_many(command(Ip::IpCloud)).await,
        ip_pools: collector.optional_many(command(Ip::IpPool)).await,
        ip_pool_used: collector.optional_many(command(Ip::IpPoolUsed)).await,
        ip_proxy: collector.optional_many(command(Ip::IpProxy)).await,
        ip_services: EndpointSnapshot::default(),
        ip_settings: collector.optional_many(command(Ip::IpSettings)).await,
        ipsec_policies: collector.optional_many(command(Ip::IpsecPolicy)).await,
        ipsec_profiles: collector.optional_many(command(Ip::IpsecProfile)).await,
        ipsec_proposals: collector.optional_many(command(Ip::IpsecProposal)).await,
        ipsec_statistics: collector.optional_many(command(Ip::IpsecStatistics)).await,
        nat_pmp: collector.optional_many(command(Ip::NatPmp)).await,
        neighbor_discovery_settings: collector.optional_many(command(Ip::NeighborDiscoverySettings)).await,
        smb: collector.optional_many(command(Ip::Smb)).await,
        smb_shares: collector.optional_many(command(Ip::SmbShare)).await,
        socks: collector.optional_many(command(Ip::Socks)).await,
        ssh: collector.optional_many(command(Ip::Ssh)).await,
        traffic_flow: collector.optional_many(command(Ip::TrafficFlow)).await,
        upnp: collector.optional_many(command(Ip::Upnp)).await,
        vrfs: collector.optional_many(command(Ip::Vrf)).await,
    })
}

/// Collect optional `/ipv6` endpoints.
pub(super) async fn collect_ipv6(collector: &EndpointCollector<'_>) -> Ipv6Snapshot {
    Ipv6Snapshot {
        ipv6_addresses: collector.optional_many(command(Ip::Ipv6Address)).await,
        ipv6_neighbors: collector.optional_many(command(Ip::Ipv6Neighbor)).await,
        ipv6_neighbor_discovery: collector.optional_many(command(Ip::Ipv6NeighborDiscovery)).await,
        ipv6_routes: collector.optional_many(command(Ip::Ipv6Route)).await,
        ipv6_settings: collector.optional_many(command(Ip::Ipv6Settings)).await,
    }
}

/// Wrap an `/ip` or `/ipv6` command in the top-level print command.
const fn command(command: Ip) -> PrintCommand {
    PrintCommand::Ip(command)
}
