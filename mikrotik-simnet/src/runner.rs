//! Topology execution lifecycle.

use std::collections::BTreeMap;
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Stdio;
use std::thread;
use std::time::Duration;

use mikrotik_client::MikroTikClient;
use mikrotik_client::MikroTikClientBuilder;
use mikrotik_client::Protocol;
use mikrotik_client::print_checks::ALL_PRINT_CHECK_COMMANDS;
use mikrotik_client::print_checks::ALL_PRINT_CHECK_METHODS;
use mikrotik_client::print_checks::PrintCheckOptions;
use mikrotik_client::print_checks::PrintCheckReport;
use mikrotik_client::types::target::Credentials;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio::time::sleep;
use tracing::debug;
use tracing::error;
use tracing::info;
use xshell::Shell;
use xshell::cmd;

use crate::CACHE_DIR;
use crate::DEFAULT_BOOT_TIMEOUT;
use crate::DEFAULT_PASSWORD;
use crate::DEFAULT_USERNAME;
use crate::IMAGES_DIR;
use crate::RUNS_DIR;
use crate::catalog::ChrArch;
use crate::catalog::chr_archive_filename;
use crate::catalog::chr_image_filename;
use crate::catalog::chr_url;
use crate::catalog::routeros_version;
use crate::catalog::validate_routeros_versions;
use crate::error::Error;
use crate::error::Result;
use crate::qemu::RuntimeTarget;
use crate::qemu::append_accelerator_args;
use crate::qemu::append_disk_args;
use crate::qemu::create_overlay;
use crate::qemu::ensure_tool;
use crate::qemu::qemu_system_binary;
use crate::topology::Check;
use crate::topology::Router;
use crate::topology::RunOptions;
use crate::topology::Topology;

/// Emit one informational event for a specific router.
macro_rules! router_info {
    ($router:expr, $($argument:tt)*) => {
        info!("{}: {}", $router, format_args!($($argument)*));
    };
}

/// Emit one debug event for a specific router.
macro_rules! router_debug {
    ($router:expr, $($argument:tt)*) => {
        debug!("{}: {}", $router, format_args!($($argument)*));
    };
}

/// Emit one error event for a specific router.
macro_rules! router_error {
    ($router:expr, $($argument:tt)*) => {
        error!("{}: {}", $router, format_args!($($argument)*));
    };
}

/// Runtime executor for one parsed topology.
pub(crate) struct SimulatedNetwork {
    /// Crate root used for `.chr-cache` state.
    root: PathBuf,
    /// Parsed deterministic topology.
    topology: Topology,
}

impl SimulatedNetwork {
    /// Build a topology executor rooted at `root`.
    pub(crate) fn new(root: PathBuf, topology: Topology) -> Self {
        Self { root, topology }
    }

    /// Prepare images, start routers, bootstrap them, run checks, and wait for shutdown.
    pub(crate) async fn run(&self, options: RunOptions) -> Result<()> {
        info!(
            "starting topology `{}` with {} router(s), {} link(s), {} check(s)",
            self.topology.name,
            self.topology.routers.len(),
            self.topology.links.len(),
            self.topology.checks.len()
        );
        validate_routeros_versions(&self.topology)?;

        let sh = Shell::new()?;
        self.prepare_state_dirs(&sh)?;
        debug!("cache directory {}", self.root.join(CACHE_DIR).display());
        ensure_tool(&sh, "curl")?;
        ensure_tool(&sh, "unzip")?;
        ensure_tool(&sh, "qemu-img")?;

        let host_arch = ChrArch::host()?;
        let run_dir = self.run_dir(&sh)?;
        let socket_dir = Self::socket_dir(&sh, &run_dir)?;
        let _socket_dir_guard = RuntimeSocketDir(socket_dir.clone());
        debug!("run directory {}", run_dir.display());
        debug!("runtime socket directory {}", socket_dir.display());
        info!("host architecture {host_arch:?}");
        let mut api_ports = allocate_api_ports(&self.topology)?;
        let mut routers = Vec::new();

        for (index, router) in self.topology.routers.iter().enumerate() {
            let api_port = api_ports.release(&router.name)?;
            let version = routeros_version(&router.version)?;
            let target = RuntimeTarget::detect(&sh, host_arch, version, self.topology.allow_software_emulation)?;
            let qemu_system = qemu_system_binary(&sh, target.guest_arch)?;
            let base_image = self.ensure_chr_image(&sh, version.version, target.guest_arch)?;
            let overlay = run_dir.join(format!("{}.qcow2", router.name));
            router_debug!(router.name, "creating overlay at {}", overlay.display());
            create_overlay(&sh, &base_image, &overlay)?;
            routers.push(PreparedRouter {
                index,
                router: router.clone(),
                api_port,
                target,
                qemu_system,
                overlay,
            });
        }
        self.write_topology_report(&run_dir, &routers)?;

        let mut running = RunningTopology::default();

        for router in &routers {
            let start_context = StartContext {
                qemu_system: &router.qemu_system,
                target: router.target,
                run_dir: &run_dir,
                socket_dir: &socket_dir,
                links: &self.topology.links,
                sh: &sh,
            };
            let child = Self::start_router(router, &start_context)?;
            running.children.push((router.router.name.clone(), child));
        }

        let clients = self.wait_for_clients(&routers).await?;
        if !self.topology.links.is_empty() {
            self.wait_for_link_interfaces(&clients).await?;
        }
        self.bootstrap(&clients).await?;
        self.run_checks(&clients, &run_dir).await?;
        if options.exit_after_checks {
            info!(
                "topology `{}` checks passed; stopping scenario because --exit-after-checks was set",
                self.topology.name
            );
            running.shutdown();
            return Ok(());
        }
        info!(
            "topology `{}` checks passed; scenario is running until Ctrl-C",
            self.topology.name
        );
        wait_for_shutdown_signal().await?;
        running.shutdown();
        Ok(())
    }

    /// Create persistent image and per-run state directories.
    fn prepare_state_dirs(&self, sh: &Shell) -> Result<()> {
        sh.create_dir(self.root.join(CACHE_DIR))?;
        sh.create_dir(self.root.join(IMAGES_DIR))?;
        sh.create_dir(self.root.join(RUNS_DIR))?;
        Ok(())
    }

    /// Create and return a unique run directory for this topology invocation.
    fn run_dir(&self, sh: &Shell) -> Result<PathBuf> {
        for _ in 0..3 {
            let timestamp = sh
                .cmd("date")
                .arg("+%Y%m%d-%H%M%S")
                .read()
                .map_err(|error| Error::Tool(format!("format local timestamp with `date`: {error}")))?;
            let path = self
                .root
                .join(RUNS_DIR)
                .join(format!("{timestamp}-{}", self.topology.name));
            if !path.exists() {
                sh.create_dir(&path)?;
                return Ok(path);
            }
            thread::sleep(Duration::from_secs(1));
        }
        Err(Error::Tool(format!(
            "could not create a unique timestamped run directory for topology `{}`",
            self.topology.name
        )))
    }

    /// Create a short path for QEMU Unix sockets.
    fn socket_dir(sh: &Shell, run_dir: &Path) -> Result<PathBuf> {
        let run_name = run_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| Error::Tool(format!("read run directory name from {}", run_dir.display())))?;
        let path = PathBuf::from("/tmp").join(format!("mikrotik-simnet-{run_name}"));
        if path.exists() {
            fs::remove_dir_all(&path)?;
        }
        sh.create_dir(&path)?;
        Ok(path)
    }

    /// Return a cached CHR raw image path, downloading and unpacking if needed.
    fn ensure_chr_image(&self, sh: &Shell, version: &str, arch: ChrArch) -> Result<PathBuf> {
        let image = self.root.join(IMAGES_DIR).join(chr_image_filename(version, arch));
        if image.exists() {
            debug!("using cached CHR {version} {arch:?} image {}", image.display());
            return Ok(image);
        }

        let archive = self.root.join(IMAGES_DIR).join(chr_archive_filename(version, arch));
        let url = chr_url(version, arch);
        info!("downloading CHR {version} {arch:?} from {url}");
        cmd!(sh, "curl -fL {url} -o {archive}")
            .run()
            .map_err(|error| Error::Tool(format!("download CHR {version} from {url}: {error}")))?;

        let archive_member = chr_image_filename(version, arch);
        debug!("unpacking {} to {}", archive_member, image.display());
        let output = cmd!(sh, "unzip -p {archive} {archive_member}")
            .output()
            .map_err(|error| Error::Tool(format!("unpack CHR image {}: {error}", archive.display())))?;
        fs::write(&image, output.stdout)?;
        fs::remove_file(&archive)?;
        debug!("removed temporary archive {}", archive.display());

        Ok(image)
    }

    /// Spawn one QEMU process for a router using the prepared overlay image.
    fn start_router(router: &PreparedRouter, context: &StartContext<'_>) -> Result<Child> {
        let router_name = &router.router.name;
        let mut args = vec![
            "-name".to_owned(),
            router_name.clone(),
            "-m".to_owned(),
            router.router.memory_mib.to_string(),
            "-smp".to_owned(),
            router.router.cpus.to_string(),
            "-display".to_owned(),
            "none".to_owned(),
            "-serial".to_owned(),
            format!(
                "file:{}",
                context.run_dir.join(format!("{router_name}.serial.log")).display()
            ),
            "-monitor".to_owned(),
            "none".to_owned(),
            "-pidfile".to_owned(),
            context.run_dir.join(format!("{router_name}.pid")).display().to_string(),
        ];

        append_accelerator_args(&mut args, context.target);
        append_disk_args(
            &mut args,
            context.target,
            &router.overlay,
            router_name,
            context.run_dir,
            context.sh,
        )?;

        let mgmt_network = management_network(router.index)?;
        let hostfwd = format!("hostfwd=tcp:127.0.0.1:{}-:8728", router.api_port);
        args.extend([
            "-netdev".to_owned(),
            format!(
                "user,id=mgmt,net={}/24,dhcpstart={},{}",
                mgmt_network.network, mgmt_network.dhcp_start, hostfwd
            ),
            "-device".to_owned(),
            format!("virtio-net-pci,netdev=mgmt,mac={},addr=0x2", mac(router.index, 0)),
        ]);
        for (link_index, link) in context.links.iter().enumerate() {
            if link.a.router == *router_name || link.b.router == *router_name {
                append_link_args(&mut args, router, context, link_index, link)?;
            }
        }

        fs::write(
            context.run_dir.join(format!("{router_name}.qemu.args")),
            format!("{} {}\n", context.qemu_system, args.join(" ")),
        )?;

        router_info!(
            router_name,
            "starting with {} on API localhost:{}",
            context.qemu_system,
            router.api_port
        );
        router_debug!(
            router_name,
            "serial log {}",
            context.run_dir.join(format!("{router_name}.serial.log")).display()
        );
        router_debug!(
            router_name,
            "QEMU log {}",
            context.run_dir.join(format!("{router_name}.qemu.log")).display()
        );

        let mut command = context.sh.cmd(context.qemu_system).args(&args).to_command();
        command.stdout(Stdio::null()).stderr(Stdio::from(fs::File::create(
            context.run_dir.join(format!("{router_name}.qemu.log")),
        )?));

        command
            .spawn()
            .map_err(|error| Error::Tool(format!("start {} with {}: {error}", router_name, context.qemu_system)))
    }

    /// Wait until every router accepts API login and return connected clients.
    async fn wait_for_clients(&self, routers: &[PreparedRouter]) -> Result<BTreeMap<String, MikroTikClient>> {
        let mut clients = BTreeMap::new();
        let mut waiters = JoinSet::new();

        for router in routers {
            let router_name = router.router.name.clone();
            let api_port = router.api_port;
            waiters.spawn(async move {
                let client = wait_for_client(&router_name, api_port).await?;
                Ok::<_, Error>((router_name, client))
            });
        }

        while let Some(result) = waiters.join_next().await {
            let (router_name, client) =
                result.map_err(|error| Error::Tool(format!("wait for router API task: {error}")))??;
            clients.insert(router_name, client);
        }

        Ok(clients)
    }

    /// Wait until every link endpoint named by the topology exists in `RouterOS`.
    async fn wait_for_link_interfaces(&self, clients: &BTreeMap<String, MikroTikClient>) -> Result<()> {
        let mut expected = BTreeMap::<String, Vec<String>>::new();
        for link in &self.topology.links {
            expected
                .entry(link.a.router.clone())
                .or_default()
                .push(link.a.interface.clone());
            expected
                .entry(link.b.router.clone())
                .or_default()
                .push(link.b.interface.clone());
        }

        let deadline = Instant::now() + DEFAULT_BOOT_TIMEOUT;
        loop {
            let mut missing = Vec::new();
            for (router, interfaces) in &expected {
                let client = clients
                    .get(router)
                    .ok_or_else(|| Error::Manifest(format!("missing connected client for `{router}`")))?;
                let rows = client.call("/interface/print", &[]).await?;
                for interface in interfaces {
                    if !rows.iter().any(|row| row.get("name") == Some(interface)) {
                        missing.push(format!("{router}:{interface}"));
                    }
                }
            }

            if missing.is_empty() {
                info!("all hotplugged link interfaces are visible");
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::Tool(format!(
                    "hotplugged link interface(s) did not appear: {}",
                    missing.join(", ")
                )));
            }

            info!("waiting for hotplugged interface(s): {}", missing.join(", "));
            sleep(Duration::from_secs(2)).await;
        }
    }

    /// Apply manifest bootstrap commands to every router.
    async fn bootstrap(&self, clients: &BTreeMap<String, MikroTikClient>) -> Result<()> {
        for router in &self.topology.routers {
            let client = clients
                .get(&router.name)
                .ok_or_else(|| Error::Manifest(format!("missing connected client for `{}`", router.name)))?;
            router_info!(router.name, "applying {} bootstrap command(s)", router.bootstrap.len());
            for command in &router.bootstrap {
                router_info!(router.name, "bootstrap {}", command.command);
                let attributes = command
                    .attributes
                    .iter()
                    .map(|attribute| (attribute.key.as_str(), attribute.value.as_deref()))
                    .collect::<Vec<_>>();
                client.call(&command.command, &attributes).await.map_err(|error| {
                    Error::Check(format!(
                        "router {} bootstrap command {}: {error}",
                        router.name, command.command
                    ))
                })?;
            }
        }
        Ok(())
    }

    /// Run manifest checks against connected routers.
    async fn run_checks(&self, clients: &BTreeMap<String, MikroTikClient>, run_dir: &Path) -> Result<()> {
        info!("running {} check(s)", self.topology.checks.len());
        let mut check_index = 0;
        while check_index < self.topology.checks.len() {
            match &self.topology.checks[check_index] {
                Check::AllPrintMethods {
                    router,
                    allow_unsupported,
                } => {
                    let mut checks = vec![(router.clone(), *allow_unsupported)];
                    check_index += 1;
                    while let Some(Check::AllPrintMethods {
                        router,
                        allow_unsupported,
                    }) = self.topology.checks.get(check_index)
                    {
                        checks.push((router.clone(), *allow_unsupported));
                        check_index += 1;
                    }
                    self.run_all_print_method_checks(clients, run_dir, &checks).await?;
                }
                Check::CommandRows {
                    router,
                    command,
                    min_rows,
                } => {
                    router_info!(router, "check command-rows: {command}");
                    let client = clients
                        .get(router)
                        .ok_or_else(|| Error::Manifest(format!("missing connected client for `{router}`")))?;
                    let rows = client.call(command, &[]).await?;
                    if rows.len() < *min_rows {
                        return Err(Error::Check(format!(
                            "router {router} command {command}: expected at least {min_rows} row(s), got {}",
                            rows.len()
                        )));
                    }
                    router_info!(router, "check command-rows passed: {} row(s)", rows.len());
                    check_index += 1;
                }
            }
        }
        Ok(())
    }

    /// Run typed print checks endpoint-first across routers.
    async fn run_all_print_method_checks(
        &self,
        clients: &BTreeMap<String, MikroTikClient>,
        run_dir: &Path,
        checks: &[(String, bool)],
    ) -> Result<()> {
        info!(
            "running all-print-methods endpoint batches across {} router(s)",
            checks.len()
        );
        let mut reports = BTreeMap::new();
        for (router, _) in checks {
            router_info!(router, "check all-print-methods");
            reports.insert(router.clone(), PrintCheckReport::default());
        }

        for command in ALL_PRINT_CHECK_COMMANDS {
            let method = command.name();
            info!(
                "check all-print-methods endpoint {method} on {} router(s)",
                checks.len()
            );
            let mut tasks = JoinSet::new();

            for (router, allow_unsupported) in checks {
                let client = clients
                    .get(router)
                    .ok_or_else(|| Error::Manifest(format!("missing connected client for `{router}`")))?
                    .clone();
                let mut options = PrintCheckOptions::new().with_router_name(router.clone());
                if *allow_unsupported {
                    options = options.with_unsupported_endpoints_allowed();
                }
                let router = router.clone();
                let command = *command;
                tasks.spawn(async move {
                    let report = Box::pin(command.run(&client, &options)).await;
                    (router, report)
                });
            }

            while let Some(result) = tasks.join_next().await {
                let (router, report) =
                    result.map_err(|error| Error::Check(format!("all-print-methods task failed: {error}")))?;
                reports
                    .get_mut(&router)
                    .expect("print-check report exists for every spawned router")
                    .append(report);
            }
        }

        let mut failure_lines = Vec::new();
        for (router, _) in checks {
            let report = reports
                .get(router)
                .expect("print-check report exists for every configured router");
            self.write_print_check_report(run_dir, router, report)?;
            let version = self.router_version(router)?;
            let router_failures = print_check_report_failures(router, version, report);
            if router_failures.is_empty() {
                router_info!(router, "check all-print-methods passed: {}", report.summary());
            } else {
                router_error!(router, "check all-print-methods failed: {}", report.summary());
                failure_lines.extend(router_failures);
            }
        }

        if !failure_lines.is_empty() {
            return Err(Error::Check(format!(
                "all-print-methods failures:\nrouter\tversion\tcommand\terror\n{}",
                failure_lines.join("\n")
            )));
        }

        Ok(())
    }

    /// Return the configured `RouterOS` version for one router.
    fn router_version(&self, router_name: &str) -> Result<&str> {
        self.topology
            .routers
            .iter()
            .find(|router| router.name == router_name)
            .map(|router| router.version.as_str())
            .ok_or_else(|| Error::Manifest(format!("missing router manifest for `{router_name}`")))
    }

    /// Write a per-router endpoint sweep report into the run artifact directory.
    fn write_print_check_report(&self, run_dir: &Path, router_name: &str, report: &PrintCheckReport) -> Result<()> {
        let router = self
            .topology
            .routers
            .iter()
            .find(|router| router.name == router_name)
            .ok_or_else(|| Error::Manifest(format!("missing router manifest for `{router_name}`")))?;
        let mut lines = vec![
            "router\tversion\tran_methods\tok_count\tskipped_count\tfailed_count\tkind\tmethod\trow_count\terror"
                .to_owned(),
            print_check_report_row(
                router_name,
                &router.version,
                report,
                "summary",
                "",
                "",
                &report.summary(),
            ),
        ];

        for method in report.attempted_methods() {
            lines.push(print_check_report_row(
                router_name,
                &router.version,
                report,
                "attempted",
                method,
                "",
                "",
            ));
        }
        for success in report.successes() {
            lines.push(print_check_report_row(
                router_name,
                &router.version,
                report,
                "ok",
                success.method(),
                &success.row_count().to_string(),
                "",
            ));
        }
        for skipped in report.skipped() {
            lines.push(print_check_report_row(
                router_name,
                &router.version,
                report,
                "skipped",
                skipped.method(),
                "",
                skipped.error(),
            ));
        }
        for failure in report.failures() {
            lines.push(print_check_report_row(
                router_name,
                &router.version,
                report,
                "failed",
                failure.method(),
                "",
                failure.error(),
            ));
        }

        let path = run_dir.join(format!("{router_name}.print-checks.tsv"));
        fs::write(&path, format!("{}\n", lines.join("\n")))?;
        router_debug!(router_name, "wrote print check report {}", path.display());
        Ok(())
    }

    /// Write the planned topology/runtime target manifest before QEMU starts.
    fn write_topology_report(&self, run_dir: &Path, routers: &[PreparedRouter]) -> Result<()> {
        let mut lines = vec![
            "topology\trouters\tlinks\tchecks\tindex\trouter\tversion\thost_arch\tguest_arch\taccelerator\tqemu_system\tapi_port\tbootstrap_commands\trouter_checks".to_owned(),
        ];

        for router in routers {
            lines.push(
                [
                    tsv_field(&self.topology.name),
                    self.topology.routers.len().to_string(),
                    self.topology.links.len().to_string(),
                    self.topology.checks.len().to_string(),
                    router.index.to_string(),
                    tsv_field(&router.router.name),
                    tsv_field(&router.router.version),
                    format!("{:?}", router.target.host_arch()),
                    format!("{:?}", router.target.guest_arch),
                    tsv_field(router.target.accelerator_name()),
                    tsv_field(&router.qemu_system),
                    router.api_port.to_string(),
                    router.router.bootstrap.len().to_string(),
                    self.check_count_for_router(&router.router.name).to_string(),
                ]
                .join("\t"),
            );
        }

        let path = run_dir.join("topology.tsv");
        fs::write(&path, format!("{}\n", lines.join("\n")))?;
        debug!("wrote topology report {}", path.display());
        Ok(())
    }

    /// Return how many manifest checks target one router.
    fn check_count_for_router(&self, router_name: &str) -> usize {
        self.topology
            .checks
            .iter()
            .filter(|check| match check {
                Check::AllPrintMethods { router, .. } | Check::CommandRows { router, .. } => router == router_name,
            })
            .count()
    }
}

/// Wait until the user requests scenario shutdown.
async fn wait_for_shutdown_signal() -> Result<()> {
    tokio::signal::ctrl_c()
        .await
        .map_err(|error| Error::Tool(format!("wait for Ctrl-C: {error}")))?;
    info!("received Ctrl-C, shutting down scenario");
    Ok(())
}

/// Build one TSV row for a print-check report.
fn print_check_report_row(
    router_name: &str,
    version: &str,
    report: &PrintCheckReport,
    kind: &str,
    method: &str,
    row_count: &str,
    error: &str,
) -> String {
    [
        tsv_field(router_name),
        tsv_field(version),
        report.ran_methods().to_string(),
        report.successes().len().to_string(),
        report.skipped().len().to_string(),
        report.failures().len().to_string(),
        tsv_field(kind),
        tsv_field(method),
        tsv_field(row_count),
        tsv_field(error),
    ]
    .join("\t")
}

/// Return all terminal failures for one router print-check report.
fn print_check_report_failures(router: &str, version: &str, report: &PrintCheckReport) -> Vec<String> {
    let mut failures = Vec::new();
    let attempted_methods = report.attempted_methods();
    if report.ran_methods() == 0 {
        failures.push(print_check_failure_row(
            router,
            version,
            "all-print-methods",
            "matched no methods",
        ));
    }
    if !attempted_methods_match_expected(&attempted_methods, ALL_PRINT_CHECK_METHODS) {
        failures.push(print_check_failure_row(
            router,
            version,
            "all-print-methods",
            &format!(
                "attempted {} method(s), expected compiled endpoint set of {} method(s)",
                attempted_methods.len(),
                ALL_PRINT_CHECK_METHODS.len()
            ),
        ));
    }
    if !report.has_complete_outcome_inventory() {
        failures.push(print_check_failure_row(
            router,
            version,
            "all-print-methods",
            &format!("outcome inventory is incomplete: {}", report.summary()),
        ));
    }

    for failure in report.failures() {
        failures.push(print_check_failure_row(
            router,
            version,
            failure.method(),
            failure.error(),
        ));
    }

    failures
}

/// Build one TSV row for the final print-check failure summary.
fn print_check_failure_row(router: &str, version: &str, command: &str, error: &str) -> String {
    [
        tsv_field(router),
        tsv_field(version),
        tsv_field(command),
        tsv_field(error),
    ]
    .join("\t")
}

/// Return whether the attempted methods exactly match the compiled endpoint inventory.
fn attempted_methods_match_expected(attempted: &[&str], expected: &[&str]) -> bool {
    attempted == expected
}

/// Escape a value for the simple TSV report format.
fn tsv_field(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\t' | '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_field_flattens_separators() {
        assert_eq!(tsv_field("line\tone\nline\rtwo"), "line one line two");
    }

    #[test]
    fn attempted_methods_must_match_expected_inventory() {
        assert!(attempted_methods_match_expected(&["a", "b"], &["a", "b"]));
        assert!(!attempted_methods_match_expected(&["a"], &["a", "b"]));
        assert!(!attempted_methods_match_expected(&["b", "a"], &["a", "b"]));
    }
}

/// Router runtime inputs prepared before QEMU is spawned.
struct PreparedRouter {
    /// Router index in manifest order.
    index: usize,
    /// Parsed router manifest.
    router: Router,
    /// Host API port assigned to this router.
    api_port: u16,
    /// Runtime target selected for this router.
    target: RuntimeTarget,
    /// QEMU system binary selected for this router.
    qemu_system: String,
    /// Per-run overlay image.
    overlay: PathBuf,
}

/// Inputs shared while spawning routers for one topology run.
struct StartContext<'a> {
    /// QEMU system binary selected for the host architecture.
    qemu_system: &'a str,
    /// Runtime target selected for this router.
    target: RuntimeTarget,
    /// Per-run directory for overlays, logs, sockets, and firmware vars.
    run_dir: &'a Path,
    /// Short temporary directory for QEMU Unix sockets.
    socket_dir: &'a Path,
    /// Topology links used to reserve hotpluggable `PCIe` slots.
    links: &'a [crate::topology::Link],
    /// Shell used to construct host commands.
    sh: &'a Shell,
}

/// Temporary QEMU socket directory removed after a simulation run.
struct RuntimeSocketDir(PathBuf);

impl Drop for RuntimeSocketDir {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            error!(
                "failed to remove runtime socket directory {}: {error}",
                self.0.display()
            );
        }
    }
}

/// Child QEMU processes for a running topology.
#[derive(Default)]
struct RunningTopology {
    /// Processes paired with router names for diagnostics.
    children: Vec<(String, Child)>,
}

impl RunningTopology {
    /// Terminate and reap all child QEMU processes.
    fn shutdown(&mut self) {
        if !self.children.is_empty() {
            info!("stopping {} router process(es)", self.children.len());
        }
        for (name, child) in &mut self.children {
            if let Err(error) = child.kill() {
                router_error!(name, "failed to stop: {error}");
            }
        }
        for (name, child) in &mut self.children {
            if let Err(error) = child.wait() {
                router_error!(name, "failed to reap: {error}");
            }
        }
    }
}

impl Drop for RunningTopology {
    fn drop(&mut self) {
        for (name, child) in &mut self.children {
            if let Err(error) = child.kill() {
                router_error!(name, "failed to stop: {error}");
            }
        }
    }
}

/// Reserved host API ports for routers in one simulated network run.
struct ApiPortAllocations {
    /// Router-name keyed API port allocations.
    ports: BTreeMap<String, ApiPortAllocation>,
}

impl ApiPortAllocations {
    /// Release the temporary reservation and return the selected host API port.
    fn release(&mut self, router_name: &str) -> Result<u16> {
        let allocation = self
            .ports
            .get_mut(router_name)
            .ok_or_else(|| Error::Manifest(format!("missing API port allocation for `{router_name}`")))?;
        let port = allocation.port;
        drop(allocation.reservation.take());
        Ok(port)
    }
}

/// One reserved host API port.
struct ApiPortAllocation {
    /// Host TCP port selected for QEMU API forwarding.
    port: u16,
    /// Temporary listener that reserves the selected port until QEMU starts.
    reservation: Option<TcpListener>,
}

/// One isolated QEMU user-mode management network.
struct ManagementNetwork {
    /// Network address assigned to QEMU SLIRP.
    network: String,
    /// First DHCP lease address used before persistent management config exists.
    dhcp_start: String,
}

/// Return a stable per-router management subnet for QEMU user networking.
fn management_network(router_index: usize) -> Result<ManagementNetwork> {
    let subnet = u8::try_from(router_index + 1)
        .map_err(|_| Error::Manifest("too many routers for management subnet allocation".to_owned()))?;
    if subnet == 255 {
        return Err(Error::Manifest(
            "too many routers for management subnet allocation".to_owned(),
        ));
    }

    Ok(ManagementNetwork {
        network: format!("10.64.{subnet}.0"),
        dhcp_start: format!("10.64.{subnet}.100"),
    })
}

/// Append QEMU arguments for one point-to-point link NIC.
fn append_link_args(
    args: &mut Vec<String>,
    router: &PreparedRouter,
    context: &StartContext<'_>,
    link_index: usize,
    link: &crate::topology::Link,
) -> Result<()> {
    let router_name = &router.router.name;
    let nic_index = link_interface_index(router_name, link)?;
    let socket_path = context.socket_dir.join(format!("link-{link_index}.sock"));
    let netdev_id = format!("link{link_index}");
    let is_listener = link.a.router == *router_name;

    args.extend([
        "-netdev".to_owned(),
        format!(
            "stream,id={netdev_id},server={},addr.type=unix,addr.path={}",
            if is_listener { "on" } else { "off" },
            socket_path.display()
        ),
    ]);
    match context.target.guest_arch {
        ChrArch::X86_64 => args.extend([
            "-device".to_owned(),
            format!(
                "virtio-net-pci,netdev={netdev_id},mac={},addr=0x{:x}",
                mac(router.index, nic_index),
                nic_index + 2
            ),
        ]),
        ChrArch::Aarch64 => args.extend([
            "-device".to_owned(),
            format!(
                "pcie-root-port,id={},chassis={},slot={},addr=0x{:x}",
                link_bus_id(link_index),
                link_index + 1,
                nic_index + 2,
                nic_index + 2
            ),
            "-device".to_owned(),
            format!(
                "virtio-net-pci,bus={},netdev={netdev_id},mac={}",
                link_bus_id(link_index),
                mac(router.index, nic_index),
            ),
        ]),
    }
    Ok(())
}

/// Allocate and reserve host API ports for all routers.
fn allocate_api_ports(topology: &Topology) -> Result<ApiPortAllocations> {
    let mut ports = BTreeMap::new();

    for router in &topology.routers {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| Error::Tool(format!("reserve dynamic API port for {}: {error}", router.name)))?;
        let port = listener
            .local_addr()
            .map_err(|error| Error::Tool(format!("read reserved API port for {}: {error}", router.name)))?
            .port();
        router_info!(
            router.name,
            "API localhost:{port} username={} password={}",
            DEFAULT_USERNAME,
            display_password(DEFAULT_PASSWORD)
        );
        ports.insert(
            router.name.clone(),
            ApiPortAllocation {
                port,
                reservation: Some(listener),
            },
        );
    }

    Ok(ApiPortAllocations { ports })
}

/// Return a readable password value for operator logs.
fn display_password(password: &str) -> &str {
    if password.is_empty() { "<empty>" } else { password }
}

/// Build a localhost API client configuration for a forwarded router port.
fn client_config(router_name: &str, api_port: u16) -> MikroTikClientBuilder {
    MikroTikClientBuilder::new(
        "127.0.0.1",
        Protocol::Api,
        Credentials {
            username: DEFAULT_USERNAME.to_owned(),
            password: Some(DEFAULT_PASSWORD.to_owned()),
        },
    )
    .with_port(api_port)
    .with_log_label(router_name)
    .with_connect_retry_timeout(DEFAULT_BOOT_TIMEOUT)
}

/// Wait until one router accepts API login and return a connected client.
async fn wait_for_client(router_name: &str, api_port: u16) -> Result<MikroTikClient> {
    let config = client_config(router_name, api_port);

    let client = MikroTikClient::connect(config).await.map_err(|error| {
        Error::Tool(format!(
            "router {router_name} did not accept API login on localhost:{api_port}: {error}"
        ))
    })?;
    router_info!(router_name, "API ready on localhost:{api_port}");
    Ok(client)
}

/// Derive a deterministic locally administered MAC address.
pub(crate) fn mac(router_index: usize, nic_index: usize) -> String {
    format!(
        "02:52:{:02x}:{:02x}:{:02x}:{:02x}",
        (router_index >> 8) & 0xff,
        router_index & 0xff,
        (nic_index >> 8) & 0xff,
        nic_index & 0xff
    )
}

/// QEMU bus id reserved for one topology link endpoint.
fn link_bus_id(link_index: usize) -> String {
    format!("link{link_index}bus")
}

/// Convert a linked `etherN` endpoint into a deterministic NIC index.
pub(crate) fn link_interface_index(router_name: &str, link: &crate::topology::Link) -> Result<usize> {
    let endpoint = if link.a.router == router_name {
        &link.a
    } else if link.b.router == router_name {
        &link.b
    } else {
        return Err(Error::Manifest(format!("link does not contain router `{router_name}`")));
    };
    endpoint
        .interface
        .strip_prefix("ether")
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| Error::Manifest(format!("interface `{}` must be named etherN", endpoint.interface)))
}
