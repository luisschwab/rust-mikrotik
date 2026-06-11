//! Shared checks for typed `RouterOS` print endpoints.

use std::fmt;

use crate::MikroTikClient;
use crate::PrintMethods;
use crate::error::Error;

/// Environment variable used by tests to filter print methods by substring.
pub const PRINT_CHECK_FILTER_ENV: &str = "MIKROTIK_LIVE_FILTER";

/// Run a list of generated typed print methods and record failures.
macro_rules! run_print_methods {
    ($client:expr, $options:expr, $report:expr, [$($method:ident,)*]) => {
        $(
            let method = stringify!($method);

            if $options.filter.matches(method) {
                $report.ran_methods += 1;
                println!("running {method}");

                match $client.$method().await {
                    Ok(rows) => println!("ok {method}: {} row(s)", rows.len()),
                    Err(error) => {
                        let failure = PrintCheckFailure::new($options.router_name.clone(), method, &error);
                        println!("failed {method}: {failure}");
                        $report.failures.push(failure);
                    }
                }
            } else {
                println!("skipping {method}: filtered out");
            }
        )*
    };
}

/// Filter for selecting print methods by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrintCheckFilter(Option<String>);

impl PrintCheckFilter {
    /// Build a filter from an optional non-empty substring.
    #[must_use]
    pub fn new(pattern: Option<String>) -> Self {
        Self(pattern.filter(|pattern| !pattern.is_empty()))
    }

    /// Build a filter from [`PRINT_CHECK_FILTER_ENV`].
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(std::env::var(PRINT_CHECK_FILTER_ENV).ok())
    }

    /// Return the configured method-name substring.
    #[must_use]
    pub fn pattern(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Return whether this filter accepts `method`.
    #[must_use]
    pub fn matches(&self, method: &str) -> bool {
        self.pattern().is_none_or(|pattern| method.contains(pattern))
    }
}

/// Options for [`run_all_print_checks`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrintCheckOptions {
    /// Router name included in failure output.
    router_name: Option<String>,
    /// Optional method-name filter.
    filter: PrintCheckFilter,
}

impl PrintCheckOptions {
    /// Build default print-check options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a router name to failures.
    #[must_use]
    pub fn with_router_name(mut self, router_name: impl Into<String>) -> Self {
        self.router_name = Some(router_name.into());
        self
    }

    /// Set the method-name filter.
    #[must_use]
    pub fn with_filter(mut self, filter: PrintCheckFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Return the configured method-name filter.
    #[must_use]
    pub const fn filter(&self) -> &PrintCheckFilter {
        &self.filter
    }
}

/// Summary returned after running typed print endpoint checks.
#[derive(Debug, Clone, Default)]
pub struct PrintCheckReport {
    /// Number of print methods that matched the filter and ran.
    ran_methods: usize,
    /// Failures collected while running matched methods.
    failures: Vec<PrintCheckFailure>,
}

impl PrintCheckReport {
    /// Return the number of print methods executed.
    #[must_use]
    pub const fn ran_methods(&self) -> usize {
        self.ran_methods
    }

    /// Return all failed print methods.
    #[must_use]
    pub fn failures(&self) -> &[PrintCheckFailure] {
        &self.failures
    }

    /// Return whether all executed methods passed.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }

    /// Panic if no methods ran or if any print method failed.
    ///
    /// # Panics
    ///
    /// Panics when the filter matched no methods or when one or more checks failed.
    pub fn assert_success(&self) {
        assert!(self.ran_methods > 0, "print endpoint filter matched no methods");
        assert!(
            self.failures.is_empty(),
            "print endpoint failures:\n{}",
            self.failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Details for one failed typed print endpoint check.
#[derive(Debug, Clone)]
pub struct PrintCheckFailure {
    /// Router name attached to this failure, when known.
    router_name: Option<String>,
    /// Typed print method that failed.
    method: String,
    /// Formatted client error.
    error: String,
}

impl PrintCheckFailure {
    /// Build a failure record.
    fn new(router_name: Option<String>, method: &str, error: &Error) -> Self {
        Self {
            router_name,
            method: method.to_owned(),
            error: error.to_string(),
        }
    }

    /// Return the router name attached to this failure.
    #[must_use]
    pub fn router_name(&self) -> Option<&str> {
        self.router_name.as_deref()
    }

    /// Return the typed print method that failed.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Return the formatted client error.
    #[must_use]
    pub fn error(&self) -> &str {
        &self.error
    }
}

impl fmt::Display for PrintCheckFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(formatter, "{router_name} ")?;
        }
        write!(formatter, "{}: {}", self.method, self.error)
    }
}

/// Run every typed print endpoint method against a connected client.
pub async fn run_all_print_checks(client: &MikroTikClient, options: &PrintCheckOptions) -> PrintCheckReport {
    let mut report = PrintCheckReport::default();

    run_interface_methods(client, options, &mut report).await;
    run_ip_methods(client, options, &mut report).await;
    run_queue_methods(client, options, &mut report).await;
    run_routing_methods(client, options, &mut report).await;
    run_service_methods(client, options, &mut report).await;
    run_snmp_methods(client, options, &mut report).await;
    run_system_methods(client, options, &mut report).await;
    run_tool_methods(client, options, &mut report).await;
    run_user_methods(client, options, &mut report).await;

    report
}

/// Run interface-family print methods.
async fn run_interface_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run IP-family print methods.
async fn run_ip_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run queue-family print methods.
async fn run_queue_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
        [queue_queue_interface_print, queue_queue_type_print,]
    );
}

/// Run routing-family print methods.
async fn run_routing_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run service-family print methods.
async fn run_service_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run SNMP-family print methods.
async fn run_snmp_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(client, options, report, [snmp_snmp_print, snmp_snmp_community_print,]);
}

/// Run system-family print methods.
async fn run_system_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run tool-family print methods.
async fn run_tool_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
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

/// Run user-family print methods.
async fn run_user_methods(client: &MikroTikClient, options: &PrintCheckOptions, report: &mut PrintCheckReport) {
    run_print_methods!(
        client,
        options,
        report,
        [
            user_active_user_print,
            user_user_print,
            user_user_aaa_print,
            user_user_group_print,
            user_user_settings_print,
        ]
    );
}
