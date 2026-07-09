//! Simulation report helpers for `RouterOS` print command checks.

use core::fmt;
use std::time::Duration;

use mikrotik_client::client::AsyncClient;
use mikrotik_client::commands::PrintCommand;
use mikrotik_client::error::Error;
use tokio::time::sleep;

/// Number of retries for transient `RouterOS` print command traps.
const PRINT_COMMAND_TRANSIENT_RETRIES: usize = 1;
/// Delay before retrying a transient `RouterOS` print command trap.
const PRINT_COMMAND_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Emit one print command event, prefixed by router name when available.
fn print_command_info(options: &PrintCommandCheckOptions, arguments: fmt::Arguments<'_>) {
    if let Some(router) = options.router_name.as_deref() {
        tracing::info!("{router}: {arguments}");
    } else {
        tracing::info!("{arguments}");
    }
}

/// Emit one print command trace event, prefixed by router name when available.
fn print_command_trace(options: &PrintCommandCheckOptions, arguments: fmt::Arguments<'_>) {
    if let Some(router) = options.router_name.as_deref() {
        tracing::trace!("{router}: {arguments}");
    } else {
        tracing::trace!("{arguments}");
    }
}

/// Run one generated print command and record its outcome.
async fn run_print_command(
    client: &AsyncClient,
    options: &PrintCommandCheckOptions,
    report: &mut PrintCommandReport,
    command: PrintCommand,
) {
    let command_name = command.to_string();
    let command_name = command_name.as_str();

    if options.filter.matches(command_name) {
        report.ran_commands += 1;
        report.attempted_commands.push(command_name.to_owned());
        print_command_info(options, format_args!("running {command_name}"));

        let mut retries = 0;
        loop {
            match mikrotik_client::print::run(client, command).await {
                Ok(row_count) => {
                    print_command_info(options, format_args!("ok {command_name}: {row_count} row(s)"));
                    report.successes.push(PrintCommandSuccess::new(
                        options.router_name.clone(),
                        command_name,
                        row_count,
                    ));
                    break;
                }
                Err(error) if is_transient_print_command_error(&error) && retries < PRINT_COMMAND_TRANSIENT_RETRIES => {
                    retries += 1;
                    print_command_info(
                        options,
                        format_args!("retrying {command_name} after transient RouterOS error: {error}"),
                    );
                    sleep(PRINT_COMMAND_RETRY_DELAY).await;
                }
                Err(error)
                    if options.allow_unsupported_commands && is_unsupported_command_error(command_name, &error) =>
                {
                    let skipped = PrintCommandSkipped::new(options.router_name.clone(), command_name, &error);
                    print_command_info(options, format_args!("skipped {command_name}: {skipped}"));
                    report.skipped.push(skipped);
                    break;
                }
                Err(error) => {
                    let failure = PrintCommandFailure::new(options.router_name.clone(), command_name, &error);
                    print_command_info(options, format_args!("failed {command_name}: {failure}"));
                    report.failures.push(failure);
                    break;
                }
            }
        }
    } else {
        print_command_trace(options, format_args!("skipping {command_name}: filtered out"));
    }
}

/// Run one print command and return a report for that command.
pub(crate) async fn run_print_command_check(
    client: &AsyncClient,
    options: &PrintCommandCheckOptions,
    command: PrintCommand,
) -> PrintCommandReport {
    let mut report = PrintCommandReport::default();
    run_print_command(client, options, &mut report, command).await;
    report
}

/// Filter for selecting print commands by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PrintCommandFilter(Option<String>);

impl PrintCommandFilter {
    /// Return the configured command-name substring.
    #[must_use]
    pub(crate) fn pattern(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Return whether this filter accepts `command`.
    #[must_use]
    pub(crate) fn matches(&self, command: &str) -> bool {
        self.pattern().is_none_or(|pattern| command.contains(pattern))
    }
}

/// Options for one print command sweep.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PrintCommandCheckOptions {
    /// Router name included in failure output.
    router_name: Option<String>,
    /// Optional command-name filter.
    filter: PrintCommandFilter,
    /// Whether version/package-specific unsupported print commands should be skipped.
    allow_unsupported_commands: bool,
}

impl PrintCommandCheckOptions {
    /// Build default print command options.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Attach a router name to failures.
    #[must_use]
    pub(crate) fn with_router_name(mut self, router_name: impl Into<String>) -> Self {
        self.router_name = Some(router_name.into());
        self
    }

    /// Skip print commands that this `RouterOS` version reports as unsupported.
    #[must_use]
    pub(crate) const fn with_unsupported_commands_allowed(mut self) -> Self {
        self.allow_unsupported_commands = true;
        self
    }
}

/// Summary returned after running print command checks.
#[derive(Debug, Clone, Default)]
pub(crate) struct PrintCommandReport {
    /// Number of print commands that matched the filter and ran.
    ran_commands: usize,
    /// Print commands attempted in report order.
    attempted_commands: Vec<String>,
    /// Successful print command calls.
    successes: Vec<PrintCommandSuccess>,
    /// Unsupported print commands skipped after this `RouterOS` version rejected them.
    skipped: Vec<PrintCommandSkipped>,
    /// Failures collected while running matched commands.
    failures: Vec<PrintCommandFailure>,
}

impl PrintCommandReport {
    /// Return the number of print commands executed.
    #[must_use]
    pub(crate) const fn ran_commands(&self) -> usize {
        self.ran_commands
    }

    /// Return all attempted print command names in report order.
    #[must_use]
    pub(crate) fn attempted_commands(&self) -> Vec<&str> {
        self.attempted_commands.iter().map(String::as_str).collect()
    }

    /// Return all successful print command calls.
    #[must_use]
    pub(crate) fn successes(&self) -> &[PrintCommandSuccess] {
        &self.successes
    }

    /// Return all unsupported print command skips.
    #[must_use]
    pub(crate) fn skipped(&self) -> &[PrintCommandSkipped] {
        &self.skipped
    }

    /// Return all failed print commands.
    #[must_use]
    pub(crate) fn failures(&self) -> &[PrintCommandFailure] {
        &self.failures
    }

    /// Append another print command report to this report.
    pub(crate) fn append(&mut self, mut other: Self) {
        self.ran_commands += other.ran_commands;
        self.attempted_commands.append(&mut other.attempted_commands);
        self.successes.append(&mut other.successes);
        self.skipped.append(&mut other.skipped);
        self.failures.append(&mut other.failures);
    }

    /// Return whether every attempted command has exactly one recorded outcome.
    #[must_use]
    pub(crate) fn has_complete_outcome_inventory(&self) -> bool {
        if self.ran_commands != self.attempted_commands.len() {
            return false;
        }

        let mut attempted = self.attempted_commands();
        let mut outcomes = self.successful_commands();
        outcomes.extend(self.skipped_commands());
        outcomes.extend(self.failed_commands());

        attempted.sort_unstable();
        outcomes.sort_unstable();
        attempted == outcomes
    }

    /// Return unsupported print command names in report order.
    #[must_use]
    pub(crate) fn skipped_commands(&self) -> Vec<&str> {
        self.skipped.iter().map(PrintCommandSkipped::command).collect()
    }

    /// Return successful print command names in report order.
    #[must_use]
    pub(crate) fn successful_commands(&self) -> Vec<&str> {
        self.successes.iter().map(PrintCommandSuccess::command).collect()
    }

    /// Return failed print command names in report order.
    #[must_use]
    pub(crate) fn failed_commands(&self) -> Vec<&str> {
        self.failures.iter().map(PrintCommandFailure::command).collect()
    }

    /// Return a compact summary suitable for CI logs.
    #[must_use]
    pub(crate) fn summary(&self) -> String {
        let mut summary = format!(
            "{} command(s), {} ok, {} skipped, {} failed",
            self.ran_commands,
            self.successes.len(),
            self.skipped.len(),
            self.failures.len()
        );

        if !self.skipped.is_empty() {
            summary.push_str("; skipped: ");
            summary.push_str(&self.skipped_commands().join(", "));
        }

        if !self.failures.is_empty() {
            summary.push_str("; failed: ");
            summary.push_str(&self.failed_commands().join(", "));
        }

        summary
    }

    /// Serialize this print command report as CSV rows.
    #[must_use]
    pub(crate) fn to_csv(&self, router_name: &str, version: &str) -> String {
        let mut lines = vec![
            "router,version,ran_commands,ok_count,skipped_count,failed_count,kind,command,row_count,error".to_owned(),
            self.csv_row(router_name, version, "summary", "", "", &self.summary()),
        ];

        for command in self.attempted_commands() {
            lines.push(self.csv_row(router_name, version, "attempted", command, "", ""));
        }
        for success in self.successes() {
            lines.push(self.csv_row(
                router_name,
                version,
                "ok",
                success.command(),
                &success.row_count().to_string(),
                "",
            ));
        }
        for skipped in self.skipped() {
            lines.push(self.csv_row(router_name, version, "skipped", skipped.command(), "", skipped.error()));
        }
        for failure in self.failures() {
            lines.push(self.csv_row(router_name, version, "failed", failure.command(), "", failure.error()));
        }

        format!("{}\n", lines.join("\n"))
    }

    /// Return all terminal failure CSV rows for this report.
    #[must_use]
    pub(crate) fn failure_rows(&self, router: &str, version: &str) -> Vec<String> {
        let mut failures = Vec::new();
        let attempted_commands = self.attempted_commands();
        if self.ran_commands() == 0 {
            failures.push(print_command_failure_row(
                router,
                version,
                "all-print-commands",
                "matched no commands",
            ));
        }
        if !attempted_commands_match_expected(
            &attempted_commands,
            PrintCommand::all().into_iter().map(|command| command.to_string()),
        ) {
            failures.push(print_command_failure_row(
                router,
                version,
                "all-print-commands",
                &format!(
                    "attempted {} command(s), expected compiled print command set of {} command(s)",
                    attempted_commands.len(),
                    PrintCommand::count()
                ),
            ));
        }
        if !self.has_complete_outcome_inventory() {
            failures.push(print_command_failure_row(
                router,
                version,
                "all-print-commands",
                &format!("outcome inventory is incomplete: {}", self.summary()),
            ));
        }

        for failure in self.failures() {
            failures.push(print_command_failure_row(
                router,
                version,
                failure.command(),
                failure.error(),
            ));
        }

        failures
    }

    /// Build one CSV row for this report.
    fn csv_row(
        &self,
        router_name: &str,
        version: &str,
        kind: &str,
        command: &str,
        row_count: &str,
        error: &str,
    ) -> String {
        [
            csv_field(router_name),
            csv_field(version),
            self.ran_commands().to_string(),
            self.successes().len().to_string(),
            self.skipped().len().to_string(),
            self.failures().len().to_string(),
            csv_field(kind),
            csv_field(command),
            csv_field(row_count),
            csv_field(error),
        ]
        .join(",")
    }
}

/// Details for one successful print command check.
#[derive(Debug, Clone)]
pub(crate) struct PrintCommandSuccess {
    /// Router name attached to this success, when known.
    router_name: Option<String>,
    /// Print command that succeeded.
    command: String,
    /// Number of rows returned by the print command.
    row_count: usize,
}

impl PrintCommandSuccess {
    /// Build a successful print command record.
    fn new(router_name: Option<String>, command: &str, row_count: usize) -> Self {
        Self {
            router_name,
            command: command.to_owned(),
            row_count,
        }
    }

    /// Return the print command that succeeded.
    #[must_use]
    pub(crate) fn command(&self) -> &str {
        &self.command
    }

    /// Return the number of rows returned by this print command.
    #[must_use]
    pub(crate) const fn row_count(&self) -> usize {
        self.row_count
    }
}

impl fmt::Display for PrintCommandSuccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(f, "{router_name} ")?;
        }
        write!(f, "{} ok: {} row(s)", self.command, self.row_count)
    }
}

/// Details for one print command skipped because `RouterOS` rejected the path.
#[derive(Debug, Clone)]
pub(crate) struct PrintCommandSkipped {
    /// Router name attached to this skip, when known.
    router_name: Option<String>,
    /// Print command that was skipped.
    command: String,
    /// Formatted client error.
    error: String,
}

impl PrintCommandSkipped {
    /// Build a skipped print command record.
    fn new(router_name: Option<String>, command: &str, error: &Error) -> Self {
        Self {
            router_name,
            command: command.to_owned(),
            error: error.to_string(),
        }
    }

    /// Return the print command that was skipped.
    #[must_use]
    pub(crate) fn command(&self) -> &str {
        &self.command
    }

    /// Return the formatted client error.
    #[must_use]
    pub(crate) fn error(&self) -> &str {
        &self.error
    }
}

impl fmt::Display for PrintCommandSkipped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(f, "{router_name} ")?;
        }
        write!(f, "{} unsupported: {}", self.command, self.error)
    }
}

/// Details for one failed print command check.
#[derive(Debug, Clone)]
pub(crate) struct PrintCommandFailure {
    /// Router name attached to this failure, when known.
    router_name: Option<String>,
    /// Print command that failed.
    command: String,
    /// Formatted client error.
    error: String,
}

impl PrintCommandFailure {
    /// Build a failure record.
    fn new(router_name: Option<String>, command: &str, error: &Error) -> Self {
        Self {
            router_name,
            command: command.to_owned(),
            error: error.to_string(),
        }
    }

    /// Return the print command that failed.
    #[must_use]
    pub(crate) fn command(&self) -> &str {
        &self.command
    }

    /// Return the formatted client error.
    #[must_use]
    pub(crate) fn error(&self) -> &str {
        &self.error
    }
}

impl fmt::Display for PrintCommandFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(router_name) = &self.router_name {
            write!(f, "{router_name} ")?;
        }
        write!(f, "{}: {}", self.command, self.error)
    }
}

/// Return whether an error is likely a transient `RouterOS` execution timeout.
fn is_transient_print_command_error(error: &Error) -> bool {
    match error {
        Error::Trap(message) => message.to_ascii_lowercase().contains("action timed out"),
        Error::Io(_)
        | Error::Connection(_)
        | Error::Login(_)
        | Error::UnsupportedProtocol(_)
        | Error::ConnectionClosed
        | Error::Fatal(_)
        | Error::Decode(_) => false,
    }
}

/// Return whether an error means the print command is unavailable in this `RouterOS`.
fn is_unsupported_command_error(command: &str, error: &Error) -> bool {
    match error {
        Error::Trap(message) => {
            let message = message.to_ascii_lowercase();
            message.contains("no such command")
                || message.contains("no such item")
                || (command == "/system/routerboard/reset-button/print" && message.contains("contact mikrotik support"))
        }
        Error::Fatal(message) => message.to_ascii_lowercase().contains("no such command"),
        Error::Io(_)
        | Error::Connection(_)
        | Error::Login(_)
        | Error::UnsupportedProtocol(_)
        | Error::ConnectionClosed
        | Error::Decode(_) => false,
    }
}

/// Build one CSV row for the final print command failure summary.
fn print_command_failure_row(router: &str, version: &str, command: &str, error: &str) -> String {
    [
        csv_field(router),
        csv_field(version),
        csv_field(command),
        csv_field(error),
    ]
    .join(",")
}

/// Return whether the attempted commands exactly match the compiled print command inventory.
fn attempted_commands_match_expected(attempted: &[&str], expected: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    let expected = expected
        .into_iter()
        .map(|command| command.as_ref().to_owned())
        .collect::<Vec<_>>();
    attempted.iter().copied().eq(expected.iter().map(String::as_str))
}

/// Escape a value for the CSV report format.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io;
    use std::path::Path;

    use mikrotik_client::builder::Builder;
    use mikrotik_client::builder::Protocol;
    use mikrotik_client::types::target::Credentials;

    use super::*;

    const PRINT_COMMAND_FILTER_ENV: &str = "MIKROTIK_LIVE_FILTER";
    const LIVE_CREDS_PATH: &str = "tests/live_router_creds.toml";
    const LIVE_CREDS_KEYS: [&str; 5] = ["address", "port", "username", "password", "protocol"];
    const LIVE_ENABLE_ENV: &str = "MIKROTIK_LIVE";

    #[test]
    fn report_summary_lists_skipped_and_failed_commands() {
        let report = PrintCommandReport {
            ran_commands: 4,
            attempted_commands: vec![
                "/ip/address/print".to_owned(),
                "/interface/wireguard/print".to_owned(),
                "/routing/bgp/template/print".to_owned(),
                "/routing/bgp/session/print".to_owned(),
            ],
            successes: vec![PrintCommandSuccess {
                router_name: Some("ros-7-23-1".to_owned()),
                command: "/ip/address/print".to_owned(),
                row_count: 3,
            }],
            skipped: vec![
                PrintCommandSkipped {
                    router_name: Some("ros-6-49-19".to_owned()),
                    command: "/interface/wireguard/print".to_owned(),
                    error: "trap: no such command".to_owned(),
                },
                PrintCommandSkipped {
                    router_name: Some("ros-6-49-19".to_owned()),
                    command: "/routing/bgp/template/print".to_owned(),
                    error: "trap: no such item".to_owned(),
                },
            ],
            failures: vec![PrintCommandFailure {
                router_name: Some("ros-7-23-1".to_owned()),
                command: "/routing/bgp/session/print".to_owned(),
                error: "decode error".to_owned(),
            }],
        };

        assert_eq!(
            report.summary(),
            "4 command(s), 1 ok, 2 skipped, 1 failed; skipped: /interface/wireguard/print, /routing/bgp/template/print; failed: /routing/bgp/session/print"
        );
        assert!(report.has_complete_outcome_inventory());
        assert_eq!(
            report.attempted_commands(),
            vec![
                "/ip/address/print",
                "/interface/wireguard/print",
                "/routing/bgp/template/print",
                "/routing/bgp/session/print"
            ]
        );
        assert_eq!(report.successful_commands(), vec!["/ip/address/print"]);
        assert_eq!(
            report.skipped_commands(),
            vec!["/interface/wireguard/print", "/routing/bgp/template/print"]
        );
        assert_eq!(report.failed_commands(), vec!["/routing/bgp/session/print"]);
    }

    #[test]
    fn report_detects_incomplete_outcome_inventory() {
        let report = PrintCommandReport {
            ran_commands: 2,
            attempted_commands: vec!["/ip/address/print".to_owned(), "/ip/route/print".to_owned()],
            successes: vec![PrintCommandSuccess {
                router_name: None,
                command: "/ip/address/print".to_owned(),
                row_count: 1,
            }],
            skipped: Vec::new(),
            failures: Vec::new(),
        };

        assert!(!report.has_complete_outcome_inventory());
    }

    #[test]
    fn report_detects_duplicate_outcome_for_missing_attempted_command() {
        let report = PrintCommandReport {
            ran_commands: 2,
            attempted_commands: vec!["/ip/address/print".to_owned(), "/ip/route/print".to_owned()],
            successes: vec![
                PrintCommandSuccess {
                    router_name: None,
                    command: "/ip/address/print".to_owned(),
                    row_count: 1,
                },
                PrintCommandSuccess {
                    router_name: None,
                    command: "/ip/address/print".to_owned(),
                    row_count: 1,
                },
            ],
            skipped: Vec::new(),
            failures: Vec::new(),
        };

        assert!(!report.has_complete_outcome_inventory());
    }

    #[test]
    fn action_timeout_is_transient_print_command_error() {
        let error = Error::Trap("action timed out - try again, if error continues contact MikroTik support".to_owned());

        assert!(is_transient_print_command_error(&error));
    }

    #[test]
    fn attempted_commands_must_match_expected_inventory() {
        assert!(attempted_commands_match_expected(&["a", "b"], ["a", "b"]));
        assert!(!attempted_commands_match_expected(&["a"], ["a", "b"]));
        assert!(!attempted_commands_match_expected(&["b", "a"], ["a", "b"]));
    }

    #[test]
    fn print_command_report_csv_quotes_separators_and_quotes() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("one,two"), "\"one,two\"");
        assert_eq!(csv_field("line\n\"two\""), "\"line\n\"\"two\"\"\"");
    }

    #[tokio::test]
    async fn live_router_print_commands() {
        if !live_enabled() {
            println!("skipping live router test: set {LIVE_ENABLE_ENV}=1 to run against {LIVE_CREDS_PATH}");
            return;
        }

        let Some(config) = live_config().expect("live router configuration should be readable") else {
            println!("skipping live router test: {LIVE_CREDS_PATH} is missing or incomplete");
            return;
        };

        let client = AsyncClient::connect(config)
            .await
            .expect("live router should accept login");
        let filter = print_command_filter_from_env();

        if let Some(pattern) = filter.pattern() {
            println!("filtering live router commands with {PRINT_COMMAND_FILTER_ENV}={pattern}");
        }

        let report = run_all_print_command_checks(
            &client,
            PrintCommandCheckOptions {
                filter,
                ..PrintCommandCheckOptions::new()
            },
        )
        .await;
        assert_print_command_success(&report);
    }

    async fn run_all_print_command_checks(
        client: &AsyncClient,
        options: PrintCommandCheckOptions,
    ) -> PrintCommandReport {
        let mut report = PrintCommandReport::default();

        for command in PrintCommand::all() {
            report.append(run_print_command_check(client, &options, command).await);
        }

        report
    }

    fn print_command_filter_from_env() -> PrintCommandFilter {
        PrintCommandFilter(
            std::env::var(PRINT_COMMAND_FILTER_ENV)
                .ok()
                .filter(|pattern| !pattern.is_empty()),
        )
    }

    fn assert_print_command_success(report: &PrintCommandReport) {
        assert!(report.ran_commands > 0, "print command filter matched no commands");
        assert!(
            report.has_complete_outcome_inventory(),
            "print command outcome inventory is incomplete: {} attempted, {} ok, {} skipped, {} failed",
            report.attempted_commands.len(),
            report.successes.len(),
            report.skipped.len(),
            report.failures.len()
        );
        assert!(
            report.failures.is_empty(),
            "print command failures:\n{}",
            report
                .failures
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    fn live_enabled() -> bool {
        std::env::var(LIVE_ENABLE_ENV).is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
    }

    fn live_config() -> io::Result<Option<Builder>> {
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
            Builder::new(
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

        fs::read_to_string(path).map(|contents| contents.lines().filter_map(parse_toml_line).collect())
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
            "ssh" => Protocol::Ssh,
            "telnet" => Protocol::Telnet,
            "ftp" => Protocol::Ftp,
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            "winbox" => Protocol::WinBox,
            "mac-telnet" => Protocol::MacTelnet,
            protocol => panic!(
                "protocol should be `api`, `api-ssl`, `ssh`, `telnet`, `ftp`, `http`, `https`, `winbox`, or `mac-telnet`, got `{protocol}`"
            ),
        }
    }
}
