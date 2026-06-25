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

use mikrotik_client::builder::Builder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::AsyncClient;
use mikrotik_client::commands::PrintCommand;
use mikrotik_client::types::target::Credentials;
use mikrotik_types::target::DeviceTarget;
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
use crate::mermaid::render_topology_mermaid;
use crate::qemu::RuntimeTarget;
use crate::qemu::append_accelerator_args;
use crate::qemu::append_disk_args;
use crate::qemu::create_overlay;
use crate::qemu::ensure_tool;
use crate::qemu::qemu_system_binary;
use crate::simulation_report::PrintCommandCheckOptions;
use crate::simulation_report::PrintCommandReport;
use crate::simulation_report::run_print_command_check;
use crate::topology::Check;
use crate::topology::Router;
use crate::topology::RunOptions;
use crate::topology::Topology;

/// Per-router print command report filename suffix.
const PRINT_COMMAND_REPORT_SUFFIX: &str = ".print-commands.csv";
/// Per-router serial console log filename suffix.
const SERIAL_LOG_SUFFIX: &str = ".serial.log";
/// Per-router QEMU command-line artifact filename suffix.
const QEMU_ARGS_SUFFIX: &str = ".qemu.args";
/// Per-router QEMU stderr log filename suffix.
const QEMU_LOG_SUFFIX: &str = ".qemu.log";
/// Per-router QEMU process id filename suffix.
const PID_FILE_SUFFIX: &str = ".pid";
/// Topology summary report filename.
const TOPOLOGY_REPORT_FILENAME: &str = "topology.csv";
/// Topology Mermaid diagram filename.
const TOPOLOGY_MERMAID_FILENAME: &str = "topology.mmd";
/// Maximum number of QEMU routers allowed to boot toward API readiness at once.
const ROUTER_START_WINDOW_SIZE: usize = 5;

/// Emit one informational event for a specific label.
macro_rules! info_with_label {
    ($label:expr, $($argument:tt)*) => {
        info!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one debug event for a specific label.
macro_rules! debug_with_label {
    ($label:expr, $($argument:tt)*) => {
        debug!("{}: {}", $label, format_args!($($argument)*));
    };
}

/// Emit one error event for a specific label.
macro_rules! error_with_label {
    ($label:expr, $($argument:tt)*) => {
        error!("{}: {}", $label, format_args!($($argument)*));
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
        let mut spawned = self.spawn(options).await?;
        if options.non_interactive {
            info!(
                "topology `{}` checks passed; stopping scenario because --non-interactive was set",
                self.topology.name
            );
            spawned.shutdown();
            return Ok(());
        }
        info!(
            "topology `{}` checks passed; scenario is running until Ctrl-C",
            self.topology.name
        );
        wait_for_shutdown_signal().await?;
        spawned.shutdown();
        Ok(())
    }

    /// Prepare images, start routers, bootstrap them, run checks, and return a live topology handle.
    pub(crate) async fn spawn(&self, options: RunOptions) -> Result<SpawnedTopology> {
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
        let socket_dir_guard = RuntimeSocketDir(socket_dir.clone());
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
            debug_with_label!(router.name, "creating overlay at {}", overlay.display());
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
        self.write_topology_mermaid(&run_dir)?;

        let mut nodes = Vec::new();
        let clients = self
            .start_routers_and_wait_for_clients(&routers, &run_dir, &socket_dir, &sh, &mut nodes)
            .await?;
        if !self.topology.links.is_empty() {
            self.wait_for_link_interfaces(&clients).await?;
        }
        self.bootstrap(&clients).await?;
        if options.run_checks {
            self.run_checks(&clients, &run_dir).await?;
        } else {
            info!("skipping topology checks because run_checks=false");
        }

        Ok(SpawnedTopology {
            name: self.topology.name.clone(),
            run_dir,
            nodes,
            _socket_dir_guard: socket_dir_guard,
        })
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
        let serial_log_path = router_artifact_path(context.run_dir, router_name, SERIAL_LOG_SUFFIX);
        let qemu_args_path = router_artifact_path(context.run_dir, router_name, QEMU_ARGS_SUFFIX);
        let qemu_log_path = router_artifact_path(context.run_dir, router_name, QEMU_LOG_SUFFIX);
        let pid_file_path = router_artifact_path(context.run_dir, router_name, PID_FILE_SUFFIX);
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
            format!("file:{}", serial_log_path.display()),
            "-monitor".to_owned(),
            "none".to_owned(),
            "-pidfile".to_owned(),
            pid_file_path.display().to_string(),
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

        fs::write(qemu_args_path, format!("{} {}\n", context.qemu_system, args.join(" ")))?;

        info_with_label!(
            router_name,
            "starting with {} on API localhost:{}",
            context.qemu_system,
            router.api_port
        );
        debug_with_label!(router_name, "serial log {}", serial_log_path.display());
        debug_with_label!(router_name, "QEMU log {}", qemu_log_path.display());

        let mut command = context.sh.cmd(context.qemu_system).args(&args).to_command();
        command
            .stdout(Stdio::null())
            .stderr(Stdio::from(fs::File::create(qemu_log_path)?));

        command
            .spawn()
            .map_err(|error| Error::Tool(format!("start {} with {}: {error}", router_name, context.qemu_system)))
    }

    /// Start routers with a bounded rolling window and wait for each one to accept API login.
    async fn start_routers_and_wait_for_clients(
        &self,
        routers: &[PreparedRouter],
        run_dir: &Path,
        socket_dir: &Path,
        sh: &Shell,
        nodes: &mut Vec<SpawnedNode>,
    ) -> Result<BTreeMap<String, AsyncClient>> {
        let mut clients = BTreeMap::new();
        let mut waiters = JoinSet::new();
        let mut next_router_index = 0;

        info!(
            "starting routers with rolling API readiness window of {} router(s)",
            ROUTER_START_WINDOW_SIZE.min(routers.len())
        );
        while next_router_index < routers.len() && waiters.len() < ROUTER_START_WINDOW_SIZE {
            let router = &routers[next_router_index];
            next_router_index += 1;
            let start_context = StartContext {
                qemu_system: &router.qemu_system,
                target: router.target,
                run_dir,
                socket_dir,
                links: &self.topology.links,
                sh,
            };
            Self::start_router_and_wait_for_client(&mut waiters, router, &start_context, nodes)?;
        }

        while let Some(result) = waiters.join_next().await {
            let (router_name, client) =
                result.map_err(|error| Error::Tool(format!("wait for router API task: {error}")))??;
            clients.insert(router_name, client);
            if next_router_index < routers.len() {
                let router = &routers[next_router_index];
                next_router_index += 1;
                let start_context = StartContext {
                    qemu_system: &router.qemu_system,
                    target: router.target,
                    run_dir,
                    socket_dir,
                    links: &self.topology.links,
                    sh,
                };
                Self::start_router_and_wait_for_client(&mut waiters, router, &start_context, nodes)?;
            }
        }

        Ok(clients)
    }

    /// Start one pending router and spawn its API readiness waiter.
    fn start_router_and_wait_for_client(
        waiters: &mut JoinSet<Result<(String, AsyncClient)>>,
        router: &PreparedRouter,
        start_context: &StartContext<'_>,
        nodes: &mut Vec<SpawnedNode>,
    ) -> Result<()> {
        let router_name = router.router.name.clone();
        let api_port = router.api_port;
        let child = Self::start_router(router, start_context)?;
        let target = DeviceTarget::new(
            format!("127.0.0.1:{api_port}"),
            DEFAULT_USERNAME,
            Some(DEFAULT_PASSWORD.to_owned()),
        )
        .map_err(|error| Error::Tool(format!("build target for router {router_name}: {error}")))?;

        nodes.push(SpawnedNode {
            name: router_name.clone(),
            api_port,
            target,
            child: Some(child),
        });

        waiters.spawn(async move {
            let client = wait_for_client(&router_name, api_port).await?;
            Ok((router_name, client))
        });

        Ok(())
    }

    /// Wait until every link endpoint named by the topology exists in `RouterOS`.
    async fn wait_for_link_interfaces(&self, clients: &BTreeMap<String, AsyncClient>) -> Result<()> {
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
    async fn bootstrap(&self, clients: &BTreeMap<String, AsyncClient>) -> Result<()> {
        for router in &self.topology.routers {
            let client = clients
                .get(&router.name)
                .ok_or_else(|| Error::Manifest(format!("missing connected client for `{}`", router.name)))?;
            info_with_label!(router.name, "applying {} bootstrap command(s)", router.bootstrap.len());
            for command in &router.bootstrap {
                info_with_label!(router.name, "bootstrap {}", command.command);
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
    async fn run_checks(&self, clients: &BTreeMap<String, AsyncClient>, run_dir: &Path) -> Result<()> {
        info!("running {} check(s)", self.topology.checks.len());
        let mut check_index = 0;
        while check_index < self.topology.checks.len() {
            match &self.topology.checks[check_index] {
                Check::AllPrintCommands {
                    router,
                    allow_unsupported,
                } => {
                    let mut checks = vec![(router.clone(), *allow_unsupported)];
                    check_index += 1;
                    while let Some(Check::AllPrintCommands {
                        router,
                        allow_unsupported,
                    }) = self.topology.checks.get(check_index)
                    {
                        checks.push((router.clone(), *allow_unsupported));
                        check_index += 1;
                    }
                    self.run_all_print_command_checks(clients, run_dir, &checks).await?;
                }
                Check::CommandRows {
                    router,
                    command,
                    min_rows,
                } => {
                    info_with_label!(router, "check command-rows: {command}");
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
                    info_with_label!(router, "check command-rows passed: {} row(s)", rows.len());
                    check_index += 1;
                }
            }
        }
        Ok(())
    }

    /// Run print commands command-first across routers.
    async fn run_all_print_command_checks(
        &self,
        clients: &BTreeMap<String, AsyncClient>,
        run_dir: &Path,
        checks: &[(String, bool)],
    ) -> Result<()> {
        info!("running all-print-commands batches across {} router(s)", checks.len());
        let mut reports = BTreeMap::new();
        for (router, _) in checks {
            info_with_label!(router, "check all-print-commands");
            reports.insert(router.clone(), PrintCommandReport::default());
        }

        for command in PrintCommand::all() {
            let command_name = command.to_string();
            info!("check all-print-commands {command_name} on {} router(s)", checks.len());
            let mut tasks = JoinSet::new();

            for (router, allow_unsupported) in checks {
                let client = clients
                    .get(router)
                    .ok_or_else(|| Error::Manifest(format!("missing connected client for `{router}`")))?
                    .clone();
                let mut options = PrintCommandCheckOptions::new().with_router_name(router.clone());
                if *allow_unsupported {
                    options = options.with_unsupported_commands_allowed();
                }
                let router = router.clone();
                tasks.spawn(async move {
                    let report = Box::pin(run_print_command_check(&client, &options, command)).await;
                    (router, report)
                });
            }

            while let Some(result) = tasks.join_next().await {
                let (router, report) =
                    result.map_err(|error| Error::Check(format!("all-print-commands task failed: {error}")))?;
                reports
                    .get_mut(&router)
                    .expect("print command report exists for every spawned router")
                    .append(report);
            }
        }

        let mut failure_lines = Vec::new();
        for (router, _) in checks {
            let report = reports
                .get(router)
                .expect("print command report exists for every configured router");
            self.write_print_command_report(run_dir, router, report)?;
            let version = self.router_version(router)?;
            let router_failures = report.failure_rows(router, version);
            if router_failures.is_empty() {
                info_with_label!(router, "check all-print-commands passed: {}", report.summary());
            } else {
                error_with_label!(router, "check all-print-commands failed: {}", report.summary());
                failure_lines.extend(router_failures);
            }
        }

        if !failure_lines.is_empty() {
            return Err(Error::Check(format!(
                "all-print-commands failures:\nrouter,version,command,error\n{}",
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

    /// Write a per-router print command sweep report into the run artifact directory.
    fn write_print_command_report(&self, run_dir: &Path, router_name: &str, report: &PrintCommandReport) -> Result<()> {
        let router = self
            .topology
            .routers
            .iter()
            .find(|router| router.name == router_name)
            .ok_or_else(|| Error::Manifest(format!("missing router manifest for `{router_name}`")))?;

        let path = run_dir.join(format!("{router_name}{PRINT_COMMAND_REPORT_SUFFIX}"));
        fs::write(&path, report.to_csv(router_name, &router.version))?;
        debug_with_label!(router_name, "wrote print command report {}", path.display());
        Ok(())
    }

    /// Write the planned topology/runtime target manifest before QEMU starts.
    fn write_topology_report(&self, run_dir: &Path, routers: &[PreparedRouter]) -> Result<()> {
        let mut lines = vec![
            "topology,routers,links,checks,index,router,version,host_arch,guest_arch,accelerator,qemu_system,api_port,bootstrap_commands,router_checks".to_owned(),
        ];

        for router in routers {
            lines.push(
                [
                    csv_field(&self.topology.name),
                    self.topology.routers.len().to_string(),
                    self.topology.links.len().to_string(),
                    self.topology.checks.len().to_string(),
                    router.index.to_string(),
                    csv_field(&router.router.name),
                    csv_field(&router.router.version),
                    format!("{:?}", router.target.host_arch()),
                    format!("{:?}", router.target.guest_arch),
                    csv_field(router.target.accelerator_name()),
                    csv_field(&router.qemu_system),
                    router.api_port.to_string(),
                    router.router.bootstrap.len().to_string(),
                    self.check_count_for_router(&router.router.name).to_string(),
                ]
                .join(","),
            );
        }

        let path = run_dir.join(TOPOLOGY_REPORT_FILENAME);
        fs::write(&path, format!("{}\n", lines.join("\n")))?;
        debug!("wrote topology report {}", path.display());
        Ok(())
    }

    /// Write a Mermaid diagram for this topology into the run artifact directory.
    fn write_topology_mermaid(&self, run_dir: &Path) -> Result<()> {
        let path = run_dir.join(TOPOLOGY_MERMAID_FILENAME);
        fs::write(&path, render_topology_mermaid(&self.topology))?;
        debug!("wrote topology Mermaid diagram {}", path.display());
        Ok(())
    }

    /// Return how many manifest checks target one router.
    fn check_count_for_router(&self, router_name: &str) -> usize {
        self.topology
            .checks
            .iter()
            .filter(|check| match check {
                Check::AllPrintCommands { router, .. } | Check::CommandRows { router, .. } => router == router_name,
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

/// Escape a value for the CSV report format.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Return the path for one per-router run artifact.
fn router_artifact_path(run_dir: &Path, router_name: &str, suffix: &str) -> PathBuf {
    run_dir.join(format!("{router_name}{suffix}"))
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

/// Live simulated topology returned by code-driven spawns.
pub struct SpawnedTopology {
    /// Topology name from the manifest.
    name: String,
    /// Per-run artifact directory.
    run_dir: PathBuf,
    /// Running simulated nodes.
    nodes: Vec<SpawnedNode>,
    /// Runtime socket directory removed after nodes are dropped.
    _socket_dir_guard: RuntimeSocketDir,
}

impl SpawnedTopology {
    /// Return the topology name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the per-run artifact directory.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Return the spawned nodes in manifest order.
    #[must_use]
    pub fn nodes(&self) -> &[SpawnedNode] {
        &self.nodes
    }

    /// Find a spawned node by router name.
    #[must_use]
    pub fn node(&self, name: &str) -> Option<&SpawnedNode> {
        self.nodes.iter().find(|node| node.name == name)
    }

    /// Stop and reap every spawned node immediately.
    pub fn shutdown(&mut self) {
        if !self.nodes.is_empty() {
            info!("stopping {} router process(es)", self.nodes.len());
        }
        for node in &mut self.nodes {
            node.shutdown();
        }
    }
}

/// One live simulated router.
pub struct SpawnedNode {
    /// Router name from the topology manifest.
    name: String,
    /// Host TCP port forwarded to the `RouterOS` API service.
    api_port: u16,
    /// Client target for this node.
    target: DeviceTarget,
    /// Owned QEMU child process.
    child: Option<Child>,
}

impl SpawnedNode {
    /// Return the router name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the host TCP port forwarded to the `RouterOS` API service.
    #[must_use]
    pub const fn api_port(&self) -> u16 {
        self.api_port
    }

    /// Return a client target for this simulated node.
    #[must_use]
    pub fn target(&self) -> &DeviceTarget {
        &self.target
    }

    /// Stop and reap this node immediately.
    pub fn shutdown(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };

        if let Err(error) = child.kill() {
            error_with_label!(self.name, "failed to stop: {error}");
        }
        if let Err(error) = child.wait() {
            error_with_label!(self.name, "failed to reap: {error}");
        }
    }
}

impl Drop for SpawnedNode {
    fn drop(&mut self) {
        self.shutdown();
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
        info_with_label!(
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
fn client_config(router_name: &str, api_port: u16) -> Builder {
    Builder::new(
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
    .with_connect_attempt_timeout(Duration::from_secs(1))
    .with_connect_retry_max_delay(Duration::from_secs(1))
}

/// Wait until one router accepts API login and return a connected client.
async fn wait_for_client(router_name: &str, api_port: u16) -> Result<AsyncClient> {
    let config = client_config(router_name, api_port);

    let start = Instant::now();
    info_with_label!(
        router_name,
        "waiting for API readiness at localhost:{api_port}, this may take a while..."
    );

    let client = AsyncClient::connect(config).await.map_err(|error| {
        Error::Tool(format!(
            "router {router_name} did not accept API login on localhost:{api_port}: {error}"
        ))
    })?;

    let elapsed = start.elapsed().as_secs();
    info_with_label!(router_name, "API ready on localhost:{api_port} after {elapsed} seconds");
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

/// QEMU bus ID reserved for one topology link endpoint.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_quotes_separators_and_quotes() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("one,two"), "\"one,two\"");
        assert_eq!(csv_field("line\n\"two\""), "\"line\n\"\"two\"\"\"");
    }
}
