//! Shared checks for typed `RouterOS` print endpoints.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::time::sleep;

use crate::MikroTikClient;
use crate::PrintMethods;
use crate::error::Error;

/// Environment variable used by tests to filter print methods by substring.
pub const PRINT_CHECK_FILTER_ENV: &str = "MIKROTIK_LIVE_FILTER";
/// Number of retries for transient `RouterOS` print command traps.
const PRINT_CHECK_TRANSIENT_RETRIES: usize = 1;
/// Delay before retrying a transient `RouterOS` print command trap.
const PRINT_CHECK_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Boxed future returned by one typed print endpoint command.
type PrintCheckFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
/// Function pointer used to run one typed print endpoint command.
type PrintCheckRunner =
    for<'a> fn(&'a MikroTikClient, &'a PrintCheckOptions, &'a mut PrintCheckReport) -> PrintCheckFuture<'a>;

/// One typed print endpoint command.
#[derive(Clone, Copy)]
pub struct PrintCheckCommand {
    /// Generated client method name.
    name: &'static str,
    /// Command runner.
    run: PrintCheckRunner,
}

impl PrintCheckCommand {
    /// Return the generated client method name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Run this command against a connected client.
    pub async fn run(self, client: &MikroTikClient, options: &PrintCheckOptions) -> PrintCheckReport {
        let mut report = PrintCheckReport::default();
        (self.run)(client, options, &mut report).await;
        report
    }
}

/// Emit one typed print-check event, prefixed by router name when available.
macro_rules! print_check_info {
    ($options:expr, $($argument:tt)*) => {{
        if let Some(router) = $options.router_name.as_deref() {
            tracing::info!("{router}: {}", format_args!($($argument)*));
        } else {
            tracing::info!("{}", format_args!($($argument)*));
        }
    }};
}

/// Emit one typed print-check trace event, prefixed by router name when available.
macro_rules! print_check_trace {
    ($options:expr, $($argument:tt)*) => {{
        if let Some(router) = $options.router_name.as_deref() {
            tracing::trace!("{router}: {}", format_args!($($argument)*));
        } else {
            tracing::trace!("{}", format_args!($($argument)*));
        }
    }};
}

/// Run one generated typed print method and record its outcome.
macro_rules! run_print_method {
    ($client:expr, $options:expr, $report:expr, $method:ident) => {{
        let method = stringify!($method);

        if $options.filter.matches(method) {
            $report.ran_methods += 1;
            $report.attempted_methods.push(method.to_owned());
            print_check_info!($options, "running {method}");

            let mut retries = 0;
            loop {
                match $client.$method().await {
                    Ok(rows) => {
                        print_check_info!($options, "ok {method}: {} row(s)", rows.len());
                        $report.successes.push(PrintCheckSuccess::new(
                            $options.router_name.clone(),
                            method,
                            rows.len(),
                        ));
                        break;
                    }
                    Err(error) if is_transient_print_check_error(&error) && retries < PRINT_CHECK_TRANSIENT_RETRIES => {
                        retries += 1;
                        print_check_info!($options, "retrying {method} after transient RouterOS error: {error}");
                        sleep(PRINT_CHECK_RETRY_DELAY).await;
                    }
                    Err(error)
                        if $options.allow_unsupported_endpoints && is_unsupported_endpoint_error(method, &error) =>
                    {
                        let skipped = PrintCheckSkipped::new($options.router_name.clone(), method, &error);
                        print_check_info!($options, "skipped {method}: {skipped}");
                        $report.skipped.push(skipped);
                        break;
                    }
                    Err(error) => {
                        let failure = PrintCheckFailure::new($options.router_name.clone(), method, &error);
                        print_check_info!($options, "failed {method}: {failure}");
                        $report.failures.push(failure);
                        break;
                    }
                }
            }
        } else {
            print_check_trace!($options, "skipping {method}: filtered out");
        }
    }};
}

/// Run a list of generated typed print methods and record failures.
macro_rules! run_print_methods {
    ($client:expr, $options:expr, $report:expr, [$($method:ident,)*]) => {
        $(
            run_print_method!($client, $options, $report, $method);
        )*
    };
}

/// Define the typed print endpoint command inventory.
macro_rules! print_check_inventory {
    ($($method:ident,)*) => {
        /// Typed print endpoint method names attempted by [`run_all_print_checks`] without a filter.
        pub const ALL_PRINT_CHECK_METHODS: &[&str] = &[$(stringify!($method),)*];

        /// Typed print endpoint commands attempted by [`run_all_print_checks`] without a filter.
        pub const ALL_PRINT_CHECK_COMMANDS: &[PrintCheckCommand] = &[
            $(
                PrintCheckCommand {
                    name: stringify!($method),
                    run: |client, options, report| Box::pin(async move {
                        run_print_method!(client, options, report, $method);
                    }),
                },
            )*
        ];
    };
}

print_check_inventory!(
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
    queue_queue_interface_print,
    queue_queue_type_print,
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
    snmp_snmp_print,
    snmp_snmp_community_print,
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
    user_active_user_print,
    user_user_print,
    user_user_aaa_print,
    user_user_group_print,
    user_user_settings_print,
);

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
    /// Whether version/package-specific unsupported endpoints should be skipped.
    allow_unsupported_endpoints: bool,
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

    /// Skip print endpoints that this `RouterOS` version reports as unsupported.
    #[must_use]
    pub const fn with_unsupported_endpoints_allowed(mut self) -> Self {
        self.allow_unsupported_endpoints = true;
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
    /// Typed print endpoint methods attempted in report order.
    attempted_methods: Vec<String>,
    /// Successful typed print endpoint calls.
    successes: Vec<PrintCheckSuccess>,
    /// Unsupported endpoints skipped after this `RouterOS` version rejected them.
    skipped: Vec<PrintCheckSkipped>,
    /// Failures collected while running matched methods.
    failures: Vec<PrintCheckFailure>,
}

impl PrintCheckReport {
    /// Return the number of print methods executed.
    #[must_use]
    pub const fn ran_methods(&self) -> usize {
        self.ran_methods
    }

    /// Return all attempted print endpoint method names in report order.
    #[must_use]
    pub fn attempted_methods(&self) -> Vec<&str> {
        self.attempted_methods.iter().map(String::as_str).collect()
    }

    /// Return all successful print endpoint calls.
    #[must_use]
    pub fn successes(&self) -> &[PrintCheckSuccess] {
        &self.successes
    }

    /// Return all unsupported endpoint skips.
    #[must_use]
    pub fn skipped(&self) -> &[PrintCheckSkipped] {
        &self.skipped
    }

    /// Return all failed print methods.
    #[must_use]
    pub fn failures(&self) -> &[PrintCheckFailure] {
        &self.failures
    }

    /// Append another print-check report to this report.
    pub fn append(&mut self, mut other: Self) {
        self.ran_methods += other.ran_methods;
        self.attempted_methods.append(&mut other.attempted_methods);
        self.successes.append(&mut other.successes);
        self.skipped.append(&mut other.skipped);
        self.failures.append(&mut other.failures);
    }

    /// Return whether all executed methods passed.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.failures.is_empty()
    }

    /// Return whether every attempted method has exactly one recorded outcome.
    #[must_use]
    pub fn has_complete_outcome_inventory(&self) -> bool {
        if self.ran_methods != self.attempted_methods.len() {
            return false;
        }

        let mut attempted = self.attempted_methods();
        let mut outcomes = self.successful_methods();
        outcomes.extend(self.skipped_methods());
        outcomes.extend(self.failed_methods());

        attempted.sort_unstable();
        outcomes.sort_unstable();
        attempted == outcomes
    }

    /// Return unsupported endpoint method names in report order.
    #[must_use]
    pub fn skipped_methods(&self) -> Vec<&str> {
        self.skipped.iter().map(PrintCheckSkipped::method).collect()
    }

    /// Return successful endpoint method names in report order.
    #[must_use]
    pub fn successful_methods(&self) -> Vec<&str> {
        self.successes.iter().map(PrintCheckSuccess::method).collect()
    }

    /// Return failed endpoint method names in report order.
    #[must_use]
    pub fn failed_methods(&self) -> Vec<&str> {
        self.failures.iter().map(PrintCheckFailure::method).collect()
    }

    /// Return a compact summary suitable for CI logs.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "{} method(s), {} ok, {} skipped, {} failed",
            self.ran_methods,
            self.successes.len(),
            self.skipped.len(),
            self.failures.len()
        );

        if !self.skipped.is_empty() {
            summary.push_str("; skipped: ");
            summary.push_str(&self.skipped_methods().join(", "));
        }

        if !self.failures.is_empty() {
            summary.push_str("; failed: ");
            summary.push_str(&self.failed_methods().join(", "));
        }

        summary
    }

    /// Panic if no methods ran or if any print method failed.
    ///
    /// # Panics
    ///
    /// Panics when the filter matched no methods or when one or more checks failed.
    pub fn assert_success(&self) {
        assert!(self.ran_methods > 0, "print endpoint filter matched no methods");
        assert!(
            self.has_complete_outcome_inventory(),
            "print endpoint outcome inventory is incomplete: {} attempted, {} ok, {} skipped, {} failed",
            self.attempted_methods.len(),
            self.successes.len(),
            self.skipped.len(),
            self.failures.len()
        );
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

/// Details for one successful typed print endpoint check.
#[derive(Debug, Clone)]
pub struct PrintCheckSuccess {
    /// Router name attached to this success, when known.
    router_name: Option<String>,
    /// Typed print method that succeeded.
    method: String,
    /// Number of rows returned by the endpoint.
    row_count: usize,
}

impl PrintCheckSuccess {
    /// Build a successful endpoint record.
    fn new(router_name: Option<String>, method: &str, row_count: usize) -> Self {
        Self {
            router_name,
            method: method.to_owned(),
            row_count,
        }
    }

    /// Return the router name attached to this success.
    #[must_use]
    pub fn router_name(&self) -> Option<&str> {
        self.router_name.as_deref()
    }

    /// Return the typed print method that succeeded.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Return the number of rows returned by this endpoint.
    #[must_use]
    pub const fn row_count(&self) -> usize {
        self.row_count
    }
}

impl fmt::Display for PrintCheckSuccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(formatter, "{router_name} ")?;
        }
        write!(formatter, "{} ok: {} row(s)", self.method, self.row_count)
    }
}

/// Details for one print endpoint skipped because `RouterOS` rejected the path.
#[derive(Debug, Clone)]
pub struct PrintCheckSkipped {
    /// Router name attached to this skip, when known.
    router_name: Option<String>,
    /// Typed print method that was skipped.
    method: String,
    /// Formatted client error.
    error: String,
}

impl PrintCheckSkipped {
    /// Build a skipped endpoint record.
    fn new(router_name: Option<String>, method: &str, error: &Error) -> Self {
        Self {
            router_name,
            method: method.to_owned(),
            error: error.to_string(),
        }
    }

    /// Return the router name attached to this skip.
    #[must_use]
    pub fn router_name(&self) -> Option<&str> {
        self.router_name.as_deref()
    }

    /// Return the typed print method that was skipped.
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

impl fmt::Display for PrintCheckSkipped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(formatter, "{router_name} ")?;
        }
        write!(formatter, "{} unsupported: {}", self.method, self.error)
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

/// Return whether an error is likely a transient `RouterOS` execution timeout.
fn is_transient_print_check_error(error: &Error) -> bool {
    match error {
        Error::Trap(message) => message.to_ascii_lowercase().contains("action timed out"),
        Error::Io(_)
        | Error::Connection(_)
        | Error::Login(_)
        | Error::ConnectionClosed
        | Error::Fatal(_)
        | Error::Decode(_) => false,
    }
}

/// Return whether an error means the endpoint is unavailable in this `RouterOS`.
fn is_unsupported_endpoint_error(method: &str, error: &Error) -> bool {
    match error {
        Error::Trap(message) => {
            let message = message.to_ascii_lowercase();
            message.contains("no such command")
                || message.contains("no such item")
                || (method == "system_routerboard_reset_button_print" && message.contains("contact mikrotik support"))
        }
        Error::Fatal(message) => message.to_ascii_lowercase().contains("no such command"),
        Error::Io(_) | Error::Connection(_) | Error::Login(_) | Error::ConnectionClosed | Error::Decode(_) => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_summary_lists_skipped_and_failed_methods() {
        let report = PrintCheckReport {
            ran_methods: 4,
            attempted_methods: vec![
                "ip_address_print".to_owned(),
                "interface_wire_guard_interface_print".to_owned(),
                "routing_bgp_template_print".to_owned(),
                "routing_bgp_session_print".to_owned(),
            ],
            successes: vec![PrintCheckSuccess {
                router_name: Some("ros-7-23-1".to_owned()),
                method: "ip_address_print".to_owned(),
                row_count: 3,
            }],
            skipped: vec![
                PrintCheckSkipped {
                    router_name: Some("ros-6-49-19".to_owned()),
                    method: "interface_wire_guard_interface_print".to_owned(),
                    error: "trap: no such command".to_owned(),
                },
                PrintCheckSkipped {
                    router_name: Some("ros-6-49-19".to_owned()),
                    method: "routing_bgp_template_print".to_owned(),
                    error: "trap: no such item".to_owned(),
                },
            ],
            failures: vec![PrintCheckFailure {
                router_name: Some("ros-7-23-1".to_owned()),
                method: "routing_bgp_session_print".to_owned(),
                error: "decode error".to_owned(),
            }],
        };

        assert_eq!(
            report.summary(),
            "4 method(s), 1 ok, 2 skipped, 1 failed; skipped: interface_wire_guard_interface_print, routing_bgp_template_print; failed: routing_bgp_session_print"
        );
        assert!(report.has_complete_outcome_inventory());
        assert_eq!(
            report.attempted_methods(),
            vec![
                "ip_address_print",
                "interface_wire_guard_interface_print",
                "routing_bgp_template_print",
                "routing_bgp_session_print"
            ]
        );
        assert_eq!(report.successful_methods(), vec!["ip_address_print"]);
        assert_eq!(
            report.skipped_methods(),
            vec!["interface_wire_guard_interface_print", "routing_bgp_template_print"]
        );
        assert_eq!(report.failed_methods(), vec!["routing_bgp_session_print"]);
    }

    #[test]
    fn report_detects_incomplete_outcome_inventory() {
        let report = PrintCheckReport {
            ran_methods: 2,
            attempted_methods: vec!["ip_address_print".to_owned(), "ip_route_print".to_owned()],
            successes: vec![PrintCheckSuccess {
                router_name: None,
                method: "ip_address_print".to_owned(),
                row_count: 1,
            }],
            skipped: Vec::new(),
            failures: Vec::new(),
        };

        assert!(!report.has_complete_outcome_inventory());
    }

    #[test]
    fn report_detects_duplicate_outcome_for_missing_attempted_method() {
        let report = PrintCheckReport {
            ran_methods: 2,
            attempted_methods: vec!["ip_address_print".to_owned(), "ip_route_print".to_owned()],
            successes: vec![
                PrintCheckSuccess {
                    router_name: None,
                    method: "ip_address_print".to_owned(),
                    row_count: 1,
                },
                PrintCheckSuccess {
                    router_name: None,
                    method: "ip_address_print".to_owned(),
                    row_count: 1,
                },
            ],
            skipped: Vec::new(),
            failures: Vec::new(),
        };

        assert!(!report.has_complete_outcome_inventory());
    }

    #[test]
    fn action_timeout_is_transient_print_check_error() {
        let error = Error::Trap("action timed out - try again, if error continues contact MikroTik support".to_owned());

        assert!(is_transient_print_check_error(&error));
    }
}
