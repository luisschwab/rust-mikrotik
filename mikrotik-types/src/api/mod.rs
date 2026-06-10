//! Typed `RouterOS` API response rows.
//!
//! Modules mirror `RouterOS` menu families. Row structs keep fields optional so
//! one versionless type can represent rows across `RouterOS` patch/minor releases
//! and device-specific configuration.

pub mod interface;
pub mod ip;
pub mod queue;
pub mod routing;
pub mod service;
pub mod snmp;
pub mod system;
pub mod tool;
pub mod user;

#[cfg(test)]
mod live_fixture_tests {
    extern crate std;

    use alloc::string::String;
    use alloc::vec::Vec;

    use serde::Deserialize;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    use self::std::eprintln;
    use self::std::fs;
    use self::std::path::PathBuf;
    use self::std::sync::OnceLock;
    use super::interface;
    use super::ip;
    use super::queue;
    use super::routing;
    use super::service;
    use super::snmp;
    use super::system;
    use super::tool;
    use super::user;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        responses: Vec<ResponseFixture>,
    }

    #[derive(Debug, Deserialize)]
    struct ResponseFixture {
        endpoint: String,
        rows: Vec<Value>,
    }

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("fixtures")
            .join("rb750r2-v7.15.2.json")
    }

    fn fixture() -> Option<&'static Fixture> {
        static FIXTURE: OnceLock<Option<Fixture>> = OnceLock::new();

        FIXTURE
            .get_or_init(|| {
                let path = fixture_path();
                let Ok(contents) = fs::read_to_string(&path) else {
                    eprintln!("skipping live fixture test: missing {}", path.display());
                    return None;
                };

                Some(serde_json::from_str(&contents).expect("live fixture JSON should parse"))
            })
            .as_ref()
    }

    fn assert_live_fixture_deserializes<T>(endpoint: &str, expected_rows: usize)
    where
        T: DeserializeOwned,
    {
        let Some(fixture) = fixture() else {
            return;
        };
        let response = fixture
            .responses
            .iter()
            .find(|response| response.endpoint == endpoint)
            .unwrap_or_else(|| panic!("missing live fixture for {endpoint}"));

        assert_eq!(
            response.rows.len(),
            expected_rows,
            "unexpected fixture row count for {endpoint}"
        );

        serde_json::from_value::<Vec<T>>(Value::Array(response.rows.clone()))
            .unwrap_or_else(|error| panic!("{endpoint} fixture failed to deserialize: {error}"));
    }

    macro_rules! live_api_fixture_test {
        ($name:ident, $type:ty, $endpoint:literal, $expected_rows:expr) => {
            #[test]
            fn $name() {
                assert_live_fixture_deserializes::<$type>($endpoint, $expected_rows);
            }
        };
    }

    live_api_fixture_test!(interface_bridge, interface::Bridge, "/interface/bridge/print", 1);
    live_api_fixture_test!(
        interface_bridge_host,
        interface::BridgeHost,
        "/interface/bridge/host/print",
        1
    );
    live_api_fixture_test!(
        interface_bridge_port,
        interface::BridgePort,
        "/interface/bridge/port/print",
        1
    );
    live_api_fixture_test!(
        interface_bridge_settings,
        interface::BridgeSettings,
        "/interface/bridge/settings/print",
        1
    );
    live_api_fixture_test!(
        interface_bridge_vlan,
        interface::BridgeVlan,
        "/interface/bridge/vlan/print",
        1
    );
    live_api_fixture_test!(
        interface_detect_internet,
        interface::DetectInternet,
        "/interface/detect-internet/print",
        1
    );
    live_api_fixture_test!(
        interface_ethernet_interface,
        interface::EthernetInterface,
        "/interface/ethernet/print",
        1
    );
    live_api_fixture_test!(
        interface_ethernet_switch,
        interface::EthernetSwitch,
        "/interface/ethernet/switch/print",
        1
    );
    live_api_fixture_test!(
        interface_ethernet_switch_port,
        interface::EthernetSwitchPort,
        "/interface/ethernet/switch/port/print",
        1
    );
    live_api_fixture_test!(
        interface_ethernet_switch_port_isolation,
        interface::EthernetSwitchPortIsolation,
        "/interface/ethernet/switch/port-isolation/print",
        1
    );
    live_api_fixture_test!(interface_interface, interface::Interface, "/interface/print", 1);
    live_api_fixture_test!(
        interface_interface_list,
        interface::InterfaceList,
        "/interface/list/print",
        1
    );
    live_api_fixture_test!(
        interface_interface_list_member,
        interface::InterfaceListMember,
        "/interface/list/member/print",
        1
    );
    live_api_fixture_test!(interface_lte_apn, interface::LteApn, "/interface/lte/apn/print", 1);
    live_api_fixture_test!(
        interface_vlan_interface,
        interface::VlanInterface,
        "/interface/vlan/print",
        1
    );
    live_api_fixture_test!(
        interface_wire_guard_interface,
        interface::WireGuardInterface,
        "/interface/wireguard/print",
        1
    );
    live_api_fixture_test!(
        interface_wire_guard_peer,
        interface::WireGuardPeer,
        "/interface/wireguard/peers/print",
        1
    );
    live_api_fixture_test!(
        interface_wireless_security_profile,
        interface::WirelessSecurityProfile,
        "/interface/wireless/security-profiles/print",
        1
    );
    live_api_fixture_test!(ip_address, ip::Address, "/ip/address/print", 1);
    live_api_fixture_test!(ip_arp_entry, ip::ArpEntry, "/ip/arp/print", 1);
    live_api_fixture_test!(ip_dhcp_client, ip::DhcpClient, "/ip/dhcp-client/print", 1);
    live_api_fixture_test!(ip_dhcp_lease, ip::DhcpLease, "/ip/dhcp-server/lease/print", 1);
    live_api_fixture_test!(ip_dhcp_server, ip::DhcpServer, "/ip/dhcp-server/print", 1);
    live_api_fixture_test!(
        ip_dhcp_server_network,
        ip::DhcpServerNetwork,
        "/ip/dhcp-server/network/print",
        1
    );
    live_api_fixture_test!(ip_dns, ip::Dns, "/ip/dns/print", 1);
    live_api_fixture_test!(ip_dns_cache_entry, ip::DnsCacheEntry, "/ip/dns/cache/print", 1);
    live_api_fixture_test!(
        ip_firewall_address_list_entry,
        ip::FirewallAddressListEntry,
        "/ip/firewall/address-list/print",
        1
    );
    live_api_fixture_test!(
        ip_firewall_connection,
        ip::FirewallConnection,
        "/ip/firewall/connection/print",
        1
    );
    live_api_fixture_test!(
        ip_firewall_connection_tracking,
        ip::FirewallConnectionTracking,
        "/ip/firewall/connection/tracking/print",
        1
    );
    live_api_fixture_test!(
        ip_firewall_rule_filter,
        ip::FirewallRule,
        "/ip/firewall/filter/print",
        1
    );
    live_api_fixture_test!(
        ip_firewall_rule_mangle,
        ip::FirewallRule,
        "/ip/firewall/mangle/print",
        1
    );
    live_api_fixture_test!(ip_firewall_rule_nat, ip::FirewallRule, "/ip/firewall/nat/print", 1);
    live_api_fixture_test!(ip_firewall_rule_raw, ip::FirewallRule, "/ip/firewall/raw/print", 1);
    live_api_fixture_test!(
        ip_firewall_service_port,
        ip::FirewallServicePort,
        "/ip/firewall/service-port/print",
        1
    );
    live_api_fixture_test!(ip_hotspot_profile, ip::HotspotProfile, "/ip/hotspot/profile/print", 1);
    live_api_fixture_test!(ip_hotspot_user, ip::HotspotUser, "/ip/hotspot/user/print", 1);
    live_api_fixture_test!(ip_ip_cloud, ip::IpCloud, "/ip/cloud/print", 1);
    live_api_fixture_test!(ip_ip_pool, ip::IpPool, "/ip/pool/print", 1);
    live_api_fixture_test!(ip_ip_pool_used, ip::IpPoolUsed, "/ip/pool/used/print", 1);
    live_api_fixture_test!(ip_ip_proxy, ip::IpProxy, "/ip/proxy/print", 1);
    live_api_fixture_test!(ip_ip_service, ip::IpService, "/ip/service/print", 1);
    live_api_fixture_test!(ip_ip_settings, ip::IpSettings, "/ip/settings/print", 1);
    live_api_fixture_test!(ip_ipsec_policy, ip::IpsecPolicy, "/ip/ipsec/policy/print", 1);
    live_api_fixture_test!(ip_ipsec_profile, ip::IpsecProfile, "/ip/ipsec/profile/print", 1);
    live_api_fixture_test!(ip_ipsec_proposal, ip::IpsecProposal, "/ip/ipsec/proposal/print", 1);
    live_api_fixture_test!(
        ip_ipsec_statistics,
        ip::IpsecStatistics,
        "/ip/ipsec/statistics/print",
        1
    );
    live_api_fixture_test!(ip_ipv6_address, ip::Ipv6Address, "/ipv6/address/print", 1);
    live_api_fixture_test!(ip_ipv6_neighbor, ip::Ipv6Neighbor, "/ipv6/neighbor/print", 1);
    live_api_fixture_test!(
        ip_ipv6_neighbor_discovery,
        ip::Ipv6NeighborDiscovery,
        "/ipv6/nd/print",
        1
    );
    live_api_fixture_test!(ip_ipv6_route, ip::Ipv6Route, "/ipv6/route/print", 1);
    live_api_fixture_test!(ip_ipv6_settings, ip::Ipv6Settings, "/ipv6/settings/print", 1);
    live_api_fixture_test!(ip_nat_pmp, ip::NatPmp, "/ip/nat-pmp/print", 1);
    live_api_fixture_test!(ip_neighbor, ip::Neighbor, "/ip/neighbor/print", 1);
    live_api_fixture_test!(
        ip_neighbor_discovery_settings,
        ip::NeighborDiscoverySettings,
        "/ip/neighbor/discovery-settings/print",
        1
    );
    live_api_fixture_test!(ip_route, ip::Route, "/ip/route/print", 1);
    live_api_fixture_test!(ip_smb, ip::Smb, "/ip/smb/print", 1);
    live_api_fixture_test!(ip_smb_share, ip::SmbShare, "/ip/smb/shares/print", 1);
    live_api_fixture_test!(ip_socks, ip::Socks, "/ip/socks/print", 1);
    live_api_fixture_test!(ip_ssh, ip::Ssh, "/ip/ssh/print", 1);
    live_api_fixture_test!(ip_traffic_flow, ip::TrafficFlow, "/ip/traffic-flow/print", 1);
    live_api_fixture_test!(ip_upnp, ip::Upnp, "/ip/upnp/print", 1);
    live_api_fixture_test!(ip_vrf, ip::Vrf, "/ip/vrf/print", 1);
    live_api_fixture_test!(
        queue_queue_interface,
        queue::QueueInterface,
        "/queue/interface/print",
        1
    );
    live_api_fixture_test!(queue_queue_type, queue::QueueType, "/queue/type/print", 1);
    live_api_fixture_test!(
        routing_bgp_session,
        routing::BgpSession,
        "/routing/bgp/session/print",
        0
    );
    live_api_fixture_test!(
        routing_bgp_template,
        routing::BgpTemplate,
        "/routing/bgp/template/print",
        1
    );
    live_api_fixture_test!(routing_igmp_proxy, routing::IgmpProxy, "/routing/igmp-proxy/print", 1);
    live_api_fixture_test!(routing_routing_id, routing::RoutingId, "/routing/id/print", 1);
    live_api_fixture_test!(
        routing_routing_nexthop,
        routing::RoutingNexthop,
        "/routing/nexthop/print",
        1
    );
    live_api_fixture_test!(routing_routing_route, routing::RoutingRoute, "/routing/route/print", 1);
    live_api_fixture_test!(
        routing_routing_settings,
        routing::RoutingSettings,
        "/routing/settings/print",
        1
    );
    live_api_fixture_test!(
        routing_routing_stats_memory,
        routing::RoutingStatsMemory,
        "/routing/stats/memory/print",
        1
    );
    live_api_fixture_test!(
        routing_routing_stats_origin,
        routing::RoutingStatsOrigin,
        "/routing/stats/origin/print",
        1
    );
    live_api_fixture_test!(
        routing_routing_stats_process,
        routing::RoutingStatsProcess,
        "/routing/stats/process/print",
        1
    );
    live_api_fixture_test!(
        routing_routing_stats_step,
        routing::RoutingStatsStep,
        "/routing/stats/step/print",
        1
    );
    live_api_fixture_test!(routing_routing_table, routing::RoutingTable, "/routing/table/print", 1);
    live_api_fixture_test!(service_caps_man_aaa, service::CapsManAaa, "/caps-man/aaa/print", 1);
    live_api_fixture_test!(
        service_caps_man_manager,
        service::CapsManManager,
        "/caps-man/manager/print",
        1
    );
    live_api_fixture_test!(
        service_caps_man_manager_interface,
        service::CapsManManagerInterface,
        "/caps-man/manager/interface/print",
        1
    );
    live_api_fixture_test!(
        service_certificate_settings,
        service::CertificateSettings,
        "/certificate/settings/print",
        1
    );
    live_api_fixture_test!(
        service_console_settings,
        service::ConsoleSettings,
        "/console/settings/print",
        1
    );
    live_api_fixture_test!(service_disk_settings, service::DiskSettings, "/disk/settings/print", 1);
    live_api_fixture_test!(service_file, service::File, "/file/print", 1);
    live_api_fixture_test!(service_mpls_settings, service::MplsSettings, "/mpls/settings/print", 1);
    live_api_fixture_test!(service_partition, service::Partition, "/partitions/print", 1);
    live_api_fixture_test!(service_ppp_aaa, service::PppAaa, "/ppp/aaa/print", 1);
    live_api_fixture_test!(service_ppp_profile, service::PppProfile, "/ppp/profile/print", 1);
    live_api_fixture_test!(
        service_radius_incoming,
        service::RadiusIncoming,
        "/radius/incoming/print",
        1
    );
    live_api_fixture_test!(snmp_snmp, snmp::Snmp, "/snmp/print", 1);
    live_api_fixture_test!(snmp_snmp_community, snmp::SnmpCommunity, "/snmp/community/print", 1);
    live_api_fixture_test!(system_clock, system::Clock, "/system/clock/print", 1);
    live_api_fixture_test!(system_device_mode, system::DeviceMode, "/system/device-mode/print", 1);
    live_api_fixture_test!(system_history_entry, system::HistoryEntry, "/system/history/print", 1);
    live_api_fixture_test!(system_identity, system::Identity, "/system/identity/print", 1);
    live_api_fixture_test!(system_led, system::Led, "/system/leds/print", 1);
    live_api_fixture_test!(system_license, system::License, "/system/license/print", 1);
    live_api_fixture_test!(system_log_entry, system::LogEntry, "/log/print", 1);
    live_api_fixture_test!(
        system_logging_action,
        system::LoggingAction,
        "/system/logging/action/print",
        1
    );
    live_api_fixture_test!(system_logging_rule, system::LoggingRule, "/system/logging/print", 1);
    live_api_fixture_test!(system_note, system::Note, "/system/note/print", 1);
    live_api_fixture_test!(system_ntp_client, system::NtpClient, "/system/ntp/client/print", 1);
    live_api_fixture_test!(system_ntp_server, system::NtpServer, "/system/ntp/server/print", 1);
    live_api_fixture_test!(system_package, system::Package, "/system/package/print", 1);
    live_api_fixture_test!(
        system_package_update,
        system::PackageUpdate,
        "/system/package/update/print",
        1
    );
    live_api_fixture_test!(system_resource, system::Resource, "/system/resource/print", 1);
    live_api_fixture_test!(
        system_resource_cpu,
        system::ResourceCpu,
        "/system/resource/cpu/print",
        1
    );
    live_api_fixture_test!(
        system_resource_irq,
        system::ResourceIrq,
        "/system/resource/irq/print",
        1
    );
    live_api_fixture_test!(
        system_resource_usb_settings,
        system::ResourceUsbSettings,
        "/system/resource/usb/settings/print",
        1
    );
    live_api_fixture_test!(system_routerboard, system::Routerboard, "/system/routerboard/print", 1);
    live_api_fixture_test!(
        system_routerboard_reset_button,
        system::RouterboardResetButton,
        "/system/routerboard/reset-button/print",
        1
    );
    live_api_fixture_test!(
        system_routerboard_settings,
        system::RouterboardSettings,
        "/system/routerboard/settings/print",
        1
    );
    live_api_fixture_test!(system_script_job, system::ScriptJob, "/system/script/job/print", 1);
    live_api_fixture_test!(
        system_upgrade_mirror,
        system::UpgradeMirror,
        "/system/upgrade/mirror/print",
        1
    );
    live_api_fixture_test!(system_watchdog, system::Watchdog, "/system/watchdog/print", 1);
    live_api_fixture_test!(
        tool_bandwidth_server,
        tool::BandwidthServer,
        "/tool/bandwidth-server/print",
        1
    );
    live_api_fixture_test!(tool_email, tool::Email, "/tool/e-mail/print", 1);
    live_api_fixture_test!(tool_graphing, tool::Graphing, "/tool/graphing/print", 1);
    live_api_fixture_test!(
        tool_mac_server_ping,
        tool::MacServerPing,
        "/tool/mac-server/ping/print",
        1
    );
    live_api_fixture_test!(tool_romon, tool::Romon, "/tool/romon/print", 1);
    live_api_fixture_test!(tool_romon_port, tool::RomonPort, "/tool/romon/port/print", 1);
    live_api_fixture_test!(tool_sms, tool::Sms, "/tool/sms/print", 1);
    live_api_fixture_test!(tool_sniffer, tool::Sniffer, "/tool/sniffer/print", 1);
    live_api_fixture_test!(
        tool_traffic_generator,
        tool::TrafficGenerator,
        "/tool/traffic-generator/print",
        1
    );
    live_api_fixture_test!(
        tool_traffic_generator_latency_distribution,
        tool::TrafficGeneratorLatencyDistribution,
        "/tool/traffic-generator/stats/latency-distribution/print",
        1
    );
    live_api_fixture_test!(user_active_user, user::ActiveUser, "/user/active/print", 1);
    live_api_fixture_test!(user_user, user::User, "/user/print", 1);
    live_api_fixture_test!(user_user_aaa, user::UserAaa, "/user/aaa/print", 1);
    live_api_fixture_test!(user_user_group, user::UserGroup, "/user/group/print", 1);
    live_api_fixture_test!(user_user_settings, user::UserSettings, "/user/settings/print", 1);
}
