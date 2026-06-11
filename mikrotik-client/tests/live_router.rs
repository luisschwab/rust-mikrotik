#![allow(missing_docs)]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

use mikrotik_client::MikroTikClient;
use mikrotik_client::MikroTikClientConfig;
use mikrotik_client::PrintMethods;
use mikrotik_client::Protocol;
use mikrotik_client::types::target::Credentials;

const LIVE_CREDS_PATH: &str = "tests/live_router_creds.toml";
const LIVE_CREDS_KEYS: [&str; 5] = ["address", "port", "username", "password", "protocol"];
const LIVE_FILTER_ENV: &str = "MIKROTIK_LIVE_FILTER";

macro_rules! run_print_methods {
    ($client:expr, $filter:expr, $failures:expr, $ran_methods:expr, [$($method:ident,)*]) => {
        $(
            let method = stringify!($method);

            if $filter.matches(method) {
                *$ran_methods += 1;
                println!("running {method}");

                match $client.$method().await {
                    Ok(rows) => println!("ok {method}: {} row(s)", rows.len()),
                    Err(error) => {
                        println!("failed {method}: {error}");
                        $failures.push(format!("{method}: {error}"));
                    }
                }
            } else {
                println!("skipping {method}: filtered out");
            }
        )*
    };
}

#[tokio::test]
async fn live_router_print_endpoints() {
    let Some(config) = live_config().expect("live router configuration should be readable") else {
        println!("skipping live router test: {LIVE_CREDS_PATH} is missing or incomplete");
        return;
    };

    let client = MikroTikClient::connect(config)
        .await
        .expect("live router should accept login");
    let filter = LiveFilter::from_env();
    let mut ran_methods = 0;
    let mut failures = Vec::new();

    if let Some(pattern) = filter.pattern() {
        println!("filtering live router methods with {LIVE_FILTER_ENV}={pattern}");
    }

    run_interface_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_ip_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_queue_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_routing_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_service_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_snmp_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_system_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_tool_methods(&client, &filter, &mut failures, &mut ran_methods).await;
    run_user_methods(&client, &filter, &mut failures, &mut ran_methods).await;

    assert!(
        ran_methods > 0,
        "live router filter matched no print methods: {}",
        filter.pattern().unwrap_or("")
    );

    assert!(
        failures.is_empty(),
        "live router print endpoint failures:\n{}",
        failures.join("\n")
    );
}

struct LiveFilter(Option<String>);

impl LiveFilter {
    fn from_env() -> Self {
        Self(env::var(LIVE_FILTER_ENV).ok().filter(|pattern| !pattern.is_empty()))
    }

    fn pattern(&self) -> Option<&str> {
        self.0.as_deref()
    }

    fn matches(&self, method: &str) -> bool {
        self.pattern().is_none_or(|pattern| method.contains(pattern))
    }
}

async fn run_interface_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            interface_bridge_print,
            interface_bridge_host_print,
            interface_bridge_port_print,
            interface_bridge_settings_print,
            interface_bridge_vlan_print,
            interface_detect_internet_print,
            interface_ethernet_interface_print,
            interface_ethernet_switch_print,
            interface_ethernet_switch_port_print,
            interface_ethernet_switch_port_isolation_print,
            interface_interface_print,
            interface_interface_list_print,
            interface_interface_list_member_print,
            interface_lte_apn_print,
            interface_vlan_interface_print,
            interface_wire_guard_interface_print,
            interface_wire_guard_peer_print,
            interface_wireless_security_profile_print,
        ]
    );
}

async fn run_ip_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            ip_address_print,
            ip_arp_entry_print,
            ip_dhcp_client_print,
            ip_dhcp_lease_print,
            ip_dhcp_server_print,
            ip_dhcp_server_network_print,
            ip_dns_print,
            ip_dns_cache_entry_print,
            ip_firewall_address_list_entry_print,
            ip_firewall_connection_print,
            ip_firewall_connection_tracking_print,
            ip_firewall_rule_filter_print,
            ip_firewall_rule_mangle_print,
            ip_firewall_rule_nat_print,
            ip_firewall_rule_raw_print,
            ip_firewall_service_port_print,
            ip_hotspot_profile_print,
            ip_hotspot_user_print,
            ip_ip_cloud_print,
            ip_ip_pool_print,
            ip_ip_pool_used_print,
            ip_ip_proxy_print,
            ip_ip_service_print,
            ip_ip_settings_print,
            ip_ipsec_policy_print,
            ip_ipsec_profile_print,
            ip_ipsec_proposal_print,
            ip_ipsec_statistics_print,
            ip_ipv6_address_print,
            ip_ipv6_neighbor_print,
            ip_ipv6_neighbor_discovery_print,
            ip_ipv6_route_print,
            ip_ipv6_settings_print,
            ip_nat_pmp_print,
            ip_neighbor_print,
            ip_neighbor_discovery_settings_print,
            ip_route_print,
            ip_smb_print,
            ip_smb_share_print,
            ip_socks_print,
            ip_ssh_print,
            ip_traffic_flow_print,
            ip_upnp_print,
            ip_vrf_print,
        ]
    );
}

async fn run_queue_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [queue_queue_interface_print, queue_queue_type_print,]
    );
}

async fn run_routing_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            routing_bgp_session_print,
            routing_bgp_template_print,
            routing_igmp_proxy_print,
            routing_routing_id_print,
            routing_routing_nexthop_print,
            routing_routing_route_print,
            routing_routing_settings_print,
            routing_routing_stats_memory_print,
            routing_routing_stats_origin_print,
            routing_routing_stats_process_print,
            routing_routing_stats_step_print,
            routing_routing_table_print,
        ]
    );
}

async fn run_service_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            service_caps_man_aaa_print,
            service_caps_man_manager_print,
            service_caps_man_manager_interface_print,
            service_certificate_settings_print,
            service_console_settings_print,
            service_disk_settings_print,
            service_file_print,
            service_mpls_settings_print,
            service_partition_print,
            service_ppp_aaa_print,
            service_ppp_profile_print,
            service_radius_incoming_print,
        ]
    );
}

async fn run_snmp_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [snmp_snmp_print, snmp_snmp_community_print,]
    );
}

async fn run_system_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            system_clock_print,
            system_device_mode_print,
            system_history_entry_print,
            system_identity_print,
            system_led_print,
            system_license_print,
            system_log_entry_print,
            system_logging_action_print,
            system_logging_rule_print,
            system_note_print,
            system_ntp_client_print,
            system_ntp_server_print,
            system_package_print,
            system_package_update_print,
            system_resource_print,
            system_resource_cpu_print,
            system_resource_irq_print,
            system_resource_usb_settings_print,
            system_routerboard_print,
            system_routerboard_reset_button_print,
            system_routerboard_settings_print,
            system_script_job_print,
            system_upgrade_mirror_print,
            system_watchdog_print,
        ]
    );
}

async fn run_tool_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            tool_bandwidth_server_print,
            tool_email_print,
            tool_graphing_print,
            tool_mac_server_ping_print,
            tool_romon_print,
            tool_romon_port_print,
            tool_sms_print,
            tool_sniffer_print,
            tool_traffic_generator_print,
            tool_traffic_generator_latency_distribution_print,
        ]
    );
}

async fn run_user_methods(
    client: &MikroTikClient,
    filter: &LiveFilter,
    failures: &mut Vec<String>,
    ran_methods: &mut usize,
) {
    run_print_methods!(
        client,
        filter,
        failures,
        ran_methods,
        [
            user_active_user_print,
            user_user_print,
            user_user_aaa_print,
            user_user_group_print,
            user_user_settings_print,
        ]
    );
}

fn live_config() -> io::Result<Option<MikroTikClientConfig>> {
    let creds = read_creds_file(Path::new(env!("CARGO_MANIFEST_DIR")).join(LIVE_CREDS_PATH))?;

    if LIVE_CREDS_KEYS.iter().any(|key| !creds.contains_key(*key)) {
        return Ok(None);
    }

    let address = creds.get("address").expect("presence checked").to_owned();
    let port = creds
        .get("port")
        .expect("presence checked")
        .parse::<u16>()
        .expect("port should be a valid TCP port");
    let username = creds.get("username").expect("presence checked").to_owned();
    let password = creds.get("password").expect("presence checked").to_owned();
    let protocol = parse_protocol(creds.get("protocol").expect("presence checked"));

    Ok(Some(
        MikroTikClientConfig::new(
            address,
            protocol,
            Credentials {
                username,
                password: Some(password),
            },
        )
        .with_port(port),
    ))
}

fn read_creds_file(path: impl AsRef<Path>) -> io::Result<BTreeMap<String, String>> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    fs::read_to_string(path).map(|contents| contents.lines().filter_map(parse_toml_line).collect::<BTreeMap<_, _>>())
}

fn parse_toml_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
        return None;
    }

    let (key, value) = line.split_once('=')?;

    Some((key.trim().to_owned(), unquote(value.trim()).to_owned()))
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(value)
}

fn parse_protocol(value: &str) -> Protocol {
    match value {
        "api" => Protocol::Api,
        "api-ssl" => Protocol::ApiSsl,
        protocol => panic!("protocol should be `api` or `api-ssl`, got `{protocol}`"),
    }
}
