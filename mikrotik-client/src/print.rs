//! Typed wrappers for `RouterOS` print commands.

use std::future::Future;

use mikrotik_types::api;

use crate::client::MikroTikClient;
use crate::commands;
use crate::error::Result;

macro_rules! print_methods {
    ($(($method:ident, $command:ident, $ty:ty),)*) => {
        /// Typed `RouterOS` print commands.
        pub trait PrintMethods {
            $(
                #[doc = concat!("Run [`commands::", stringify!($command), "`] and decode the reply rows.")]
                fn $method(&self) -> impl Future<Output = Result<Vec<$ty>>> + Send + '_;
            )*
        }

        impl PrintMethods for MikroTikClient {
            $(
                fn $method(&self) -> impl Future<Output = Result<Vec<$ty>>> + Send + '_ {
                    self.print_typed(commands::$command)
                }
            )*
        }
    };
}

print_methods! {
    (interface_bridge_print, INTERFACE_BRIDGE_PRINT, api::interface::Bridge),
    (interface_bridge_host_print, INTERFACE_BRIDGE_HOST_PRINT, api::interface::BridgeHost),
    (interface_bridge_port_print, INTERFACE_BRIDGE_PORT_PRINT, api::interface::BridgePort),
    (interface_bridge_settings_print, INTERFACE_BRIDGE_SETTINGS_PRINT, api::interface::BridgeSettings),
    (interface_bridge_vlan_print, INTERFACE_BRIDGE_VLAN_PRINT, api::interface::BridgeVlan),
    (interface_detect_internet_print, INTERFACE_DETECT_INTERNET_PRINT, api::interface::DetectInternet),
    (interface_ethernet_interface_print, INTERFACE_ETHERNET_INTERFACE_PRINT, api::interface::EthernetInterface),
    (interface_ethernet_switch_print, INTERFACE_ETHERNET_SWITCH_PRINT, api::interface::EthernetSwitch),
    (interface_ethernet_switch_port_print, INTERFACE_ETHERNET_SWITCH_PORT_PRINT, api::interface::EthernetSwitchPort),
    (interface_ethernet_switch_port_isolation_print, INTERFACE_ETHERNET_SWITCH_PORT_ISOLATION_PRINT, api::interface::EthernetSwitchPortIsolation),
    (interface_interface_print, INTERFACE_INTERFACE_PRINT, api::interface::Interface),
    (interface_interface_list_print, INTERFACE_INTERFACE_LIST_PRINT, api::interface::InterfaceList),
    (interface_interface_list_member_print, INTERFACE_INTERFACE_LIST_MEMBER_PRINT, api::interface::InterfaceListMember),
    (interface_lte_apn_print, INTERFACE_LTE_APN_PRINT, api::interface::LteApn),
    (interface_vlan_interface_print, INTERFACE_VLAN_INTERFACE_PRINT, api::interface::VlanInterface),
    (interface_wire_guard_interface_print, INTERFACE_WIRE_GUARD_INTERFACE_PRINT, api::interface::WireGuardInterface),
    (interface_wire_guard_peer_print, INTERFACE_WIRE_GUARD_PEER_PRINT, api::interface::WireGuardPeer),
    (interface_wireless_security_profile_print, INTERFACE_WIRELESS_SECURITY_PROFILE_PRINT, api::interface::WirelessSecurityProfile),
    (ip_address_print, IP_ADDRESS_PRINT, api::ip::Address),
    (ip_arp_entry_print, IP_ARP_ENTRY_PRINT, api::ip::ArpEntry),
    (ip_dhcp_client_print, IP_DHCP_CLIENT_PRINT, api::ip::DhcpClient),
    (ip_dhcp_lease_print, IP_DHCP_LEASE_PRINT, api::ip::DhcpLease),
    (ip_dhcp_server_print, IP_DHCP_SERVER_PRINT, api::ip::DhcpServer),
    (ip_dhcp_server_network_print, IP_DHCP_SERVER_NETWORK_PRINT, api::ip::DhcpServerNetwork),
    (ip_dns_print, IP_DNS_PRINT, api::ip::Dns),
    (ip_dns_cache_entry_print, IP_DNS_CACHE_ENTRY_PRINT, api::ip::DnsCacheEntry),
    (ip_firewall_address_list_entry_print, IP_FIREWALL_ADDRESS_LIST_ENTRY_PRINT, api::ip::FirewallAddressListEntry),
    (ip_firewall_connection_print, IP_FIREWALL_CONNECTION_PRINT, api::ip::FirewallConnection),
    (ip_firewall_connection_tracking_print, IP_FIREWALL_CONNECTION_TRACKING_PRINT, api::ip::FirewallConnectionTracking),
    (ip_firewall_rule_filter_print, IP_FIREWALL_RULE_FILTER_PRINT, api::ip::FirewallRule),
    (ip_firewall_rule_mangle_print, IP_FIREWALL_RULE_MANGLE_PRINT, api::ip::FirewallRule),
    (ip_firewall_rule_nat_print, IP_FIREWALL_RULE_NAT_PRINT, api::ip::FirewallRule),
    (ip_firewall_rule_raw_print, IP_FIREWALL_RULE_RAW_PRINT, api::ip::FirewallRule),
    (ip_firewall_service_port_print, IP_FIREWALL_SERVICE_PORT_PRINT, api::ip::FirewallServicePort),
    (ip_hotspot_profile_print, IP_HOTSPOT_PROFILE_PRINT, api::ip::HotspotProfile),
    (ip_hotspot_user_print, IP_HOTSPOT_USER_PRINT, api::ip::HotspotUser),
    (ip_ip_cloud_print, IP_IP_CLOUD_PRINT, api::ip::IpCloud),
    (ip_ip_pool_print, IP_IP_POOL_PRINT, api::ip::IpPool),
    (ip_ip_pool_used_print, IP_IP_POOL_USED_PRINT, api::ip::IpPoolUsed),
    (ip_ip_proxy_print, IP_IP_PROXY_PRINT, api::ip::IpProxy),
    (ip_ip_service_print, IP_IP_SERVICE_PRINT, api::ip::IpService),
    (ip_ip_settings_print, IP_IP_SETTINGS_PRINT, api::ip::IpSettings),
    (ip_ipsec_policy_print, IP_IPSEC_POLICY_PRINT, api::ip::IpsecPolicy),
    (ip_ipsec_profile_print, IP_IPSEC_PROFILE_PRINT, api::ip::IpsecProfile),
    (ip_ipsec_proposal_print, IP_IPSEC_PROPOSAL_PRINT, api::ip::IpsecProposal),
    (ip_ipsec_statistics_print, IP_IPSEC_STATISTICS_PRINT, api::ip::IpsecStatistics),
    (ip_ipv6_address_print, IP_IPV6_ADDRESS_PRINT, api::ip::Ipv6Address),
    (ip_ipv6_neighbor_print, IP_IPV6_NEIGHBOR_PRINT, api::ip::Ipv6Neighbor),
    (ip_ipv6_neighbor_discovery_print, IP_IPV6_NEIGHBOR_DISCOVERY_PRINT, api::ip::Ipv6NeighborDiscovery),
    (ip_ipv6_route_print, IP_IPV6_ROUTE_PRINT, api::ip::Ipv6Route),
    (ip_ipv6_settings_print, IP_IPV6_SETTINGS_PRINT, api::ip::Ipv6Settings),
    (ip_nat_pmp_print, IP_NAT_PMP_PRINT, api::ip::NatPmp),
    (ip_neighbor_print, IP_NEIGHBOR_PRINT, api::ip::Neighbor),
    (ip_neighbor_discovery_settings_print, IP_NEIGHBOR_DISCOVERY_SETTINGS_PRINT, api::ip::NeighborDiscoverySettings),
    (ip_route_print, IP_ROUTE_PRINT, api::ip::Route),
    (ip_smb_print, IP_SMB_PRINT, api::ip::Smb),
    (ip_smb_share_print, IP_SMB_SHARE_PRINT, api::ip::SmbShare),
    (ip_socks_print, IP_SOCKS_PRINT, api::ip::Socks),
    (ip_ssh_print, IP_SSH_PRINT, api::ip::Ssh),
    (ip_traffic_flow_print, IP_TRAFFIC_FLOW_PRINT, api::ip::TrafficFlow),
    (ip_upnp_print, IP_UPNP_PRINT, api::ip::Upnp),
    (ip_vrf_print, IP_VRF_PRINT, api::ip::Vrf),
    (queue_queue_interface_print, QUEUE_QUEUE_INTERFACE_PRINT, api::queue::QueueInterface),
    (queue_queue_type_print, QUEUE_QUEUE_TYPE_PRINT, api::queue::QueueType),
    (routing_bgp_session_print, ROUTING_BGP_SESSION_PRINT, api::routing::BgpSession),
    (routing_bgp_template_print, ROUTING_BGP_TEMPLATE_PRINT, api::routing::BgpTemplate),
    (routing_igmp_proxy_print, ROUTING_IGMP_PROXY_PRINT, api::routing::IgmpProxy),
    (routing_routing_id_print, ROUTING_ROUTING_ID_PRINT, api::routing::RoutingId),
    (routing_routing_nexthop_print, ROUTING_ROUTING_NEXTHOP_PRINT, api::routing::RoutingNexthop),
    (routing_routing_route_print, ROUTING_ROUTING_ROUTE_PRINT, api::routing::RoutingRoute),
    (routing_routing_settings_print, ROUTING_ROUTING_SETTINGS_PRINT, api::routing::RoutingSettings),
    (routing_routing_stats_memory_print, ROUTING_ROUTING_STATS_MEMORY_PRINT, api::routing::RoutingStatsMemory),
    (routing_routing_stats_origin_print, ROUTING_ROUTING_STATS_ORIGIN_PRINT, api::routing::RoutingStatsOrigin),
    (routing_routing_stats_process_print, ROUTING_ROUTING_STATS_PROCESS_PRINT, api::routing::RoutingStatsProcess),
    (routing_routing_stats_step_print, ROUTING_ROUTING_STATS_STEP_PRINT, api::routing::RoutingStatsStep),
    (routing_routing_table_print, ROUTING_ROUTING_TABLE_PRINT, api::routing::RoutingTable),
    (service_caps_man_aaa_print, SERVICE_CAPS_MAN_AAA_PRINT, api::service::CapsManAaa),
    (service_caps_man_manager_print, SERVICE_CAPS_MAN_MANAGER_PRINT, api::service::CapsManManager),
    (service_caps_man_manager_interface_print, SERVICE_CAPS_MAN_MANAGER_INTERFACE_PRINT, api::service::CapsManManagerInterface),
    (service_certificate_settings_print, SERVICE_CERTIFICATE_SETTINGS_PRINT, api::service::CertificateSettings),
    (service_console_settings_print, SERVICE_CONSOLE_SETTINGS_PRINT, api::service::ConsoleSettings),
    (service_disk_settings_print, SERVICE_DISK_SETTINGS_PRINT, api::service::DiskSettings),
    (service_file_print, SERVICE_FILE_PRINT, api::service::File),
    (service_mpls_settings_print, SERVICE_MPLS_SETTINGS_PRINT, api::service::MplsSettings),
    (service_partition_print, SERVICE_PARTITION_PRINT, api::service::Partition),
    (service_ppp_aaa_print, SERVICE_PPP_AAA_PRINT, api::service::PppAaa),
    (service_ppp_profile_print, SERVICE_PPP_PROFILE_PRINT, api::service::PppProfile),
    (service_radius_incoming_print, SERVICE_RADIUS_INCOMING_PRINT, api::service::RadiusIncoming),
    (snmp_snmp_print, SNMP_SNMP_PRINT, api::snmp::Snmp),
    (snmp_snmp_community_print, SNMP_SNMP_COMMUNITY_PRINT, api::snmp::SnmpCommunity),
    (system_clock_print, SYSTEM_CLOCK_PRINT, api::system::Clock),
    (system_device_mode_print, SYSTEM_DEVICE_MODE_PRINT, api::system::DeviceMode),
    (system_history_entry_print, SYSTEM_HISTORY_ENTRY_PRINT, api::system::HistoryEntry),
    (system_identity_print, SYSTEM_IDENTITY_PRINT, api::system::Identity),
    (system_led_print, SYSTEM_LED_PRINT, api::system::Led),
    (system_license_print, SYSTEM_LICENSE_PRINT, api::system::License),
    (system_log_entry_print, SYSTEM_LOG_ENTRY_PRINT, api::system::LogEntry),
    (system_logging_action_print, SYSTEM_LOGGING_ACTION_PRINT, api::system::LoggingAction),
    (system_logging_rule_print, SYSTEM_LOGGING_RULE_PRINT, api::system::LoggingRule),
    (system_note_print, SYSTEM_NOTE_PRINT, api::system::Note),
    (system_ntp_client_print, SYSTEM_NTP_CLIENT_PRINT, api::system::NtpClient),
    (system_ntp_server_print, SYSTEM_NTP_SERVER_PRINT, api::system::NtpServer),
    (system_package_print, SYSTEM_PACKAGE_PRINT, api::system::Package),
    (system_package_update_print, SYSTEM_PACKAGE_UPDATE_PRINT, api::system::PackageUpdate),
    (system_resource_print, SYSTEM_RESOURCE_PRINT, api::system::Resource),
    (system_resource_cpu_print, SYSTEM_RESOURCE_CPU_PRINT, api::system::ResourceCpu),
    (system_resource_irq_print, SYSTEM_RESOURCE_IRQ_PRINT, api::system::ResourceIrq),
    (system_resource_usb_settings_print, SYSTEM_RESOURCE_USB_SETTINGS_PRINT, api::system::ResourceUsbSettings),
    (system_routerboard_print, SYSTEM_ROUTERBOARD_PRINT, api::system::Routerboard),
    (system_routerboard_reset_button_print, SYSTEM_ROUTERBOARD_RESET_BUTTON_PRINT, api::system::RouterboardResetButton),
    (system_routerboard_settings_print, SYSTEM_ROUTERBOARD_SETTINGS_PRINT, api::system::RouterboardSettings),
    (system_script_job_print, SYSTEM_SCRIPT_JOB_PRINT, api::system::ScriptJob),
    (system_upgrade_mirror_print, SYSTEM_UPGRADE_MIRROR_PRINT, api::system::UpgradeMirror),
    (system_watchdog_print, SYSTEM_WATCHDOG_PRINT, api::system::Watchdog),
    (tool_bandwidth_server_print, TOOL_BANDWIDTH_SERVER_PRINT, api::tool::BandwidthServer),
    (tool_email_print, TOOL_EMAIL_PRINT, api::tool::Email),
    (tool_graphing_print, TOOL_GRAPHING_PRINT, api::tool::Graphing),
    (tool_mac_server_ping_print, TOOL_MAC_SERVER_PING_PRINT, api::tool::MacServerPing),
    (tool_romon_print, TOOL_ROMON_PRINT, api::tool::Romon),
    (tool_romon_port_print, TOOL_ROMON_PORT_PRINT, api::tool::RomonPort),
    (tool_sms_print, TOOL_SMS_PRINT, api::tool::Sms),
    (tool_sniffer_print, TOOL_SNIFFER_PRINT, api::tool::Sniffer),
    (tool_traffic_generator_print, TOOL_TRAFFIC_GENERATOR_PRINT, api::tool::TrafficGenerator),
    (tool_traffic_generator_latency_distribution_print, TOOL_TRAFFIC_GENERATOR_LATENCY_DISTRIBUTION_PRINT, api::tool::TrafficGeneratorLatencyDistribution),
    (user_active_user_print, USER_ACTIVE_USER_PRINT, api::user::ActiveUser),
    (user_user_print, USER_USER_PRINT, api::user::User),
    (user_user_aaa_print, USER_USER_AAA_PRINT, api::user::UserAaa),
    (user_user_group_print, USER_USER_GROUP_PRINT, api::user::UserGroup),
    (user_user_settings_print, USER_USER_SETTINGS_PRINT, api::user::UserSettings),
}

#[cfg(test)]
mod tests {
    use mikrotik_types::Row;

    use super::*;

    #[test]
    fn representative_rows_decode_into_typed_api_models() {
        let route = Row::from([
            (".id".to_owned(), "*1".to_owned()),
            ("dst-address".to_owned(), "0.0.0.0/0".to_owned()),
            ("gateway".to_owned(), "192.0.2.1".to_owned()),
        ]);
        let resource = Row::from([
            ("version".to_owned(), "7.15.2".to_owned()),
            ("uptime".to_owned(), "1d2h3m4s".to_owned()),
        ]);

        let route: api::ip::Route = mikrotik_types::deserialize(&route).unwrap();
        let resource: api::system::Resource = mikrotik_types::deserialize(&resource).unwrap();

        assert_eq!(
            route.dst_address.as_ref().map(ToString::to_string).as_deref(),
            Some("0.0.0.0/0")
        );
        assert_eq!(
            resource.version.as_ref().map(ToString::to_string).as_deref(),
            Some("7.15.2")
        );
    }
}
