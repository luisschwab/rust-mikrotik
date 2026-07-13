//! Rust-first local QEMU/CHR simulation harness for `rust-mikrotik`.
//!
//! This crate exposes CHR devices and scenarios as ordinary Rust objects. A
//! [`MikrotikD`] or [`Scenario`] owns its QEMU child processes, provides
//! localhost `RouterOS` API targets, and stops processes on drop.

use core::time::Duration;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::path::PathBuf;
use std::thread;

use mikrotik_client::client::Client;
use mikrotik_common::debug_with_label;
use mikrotik_common::error_with_label;
use mikrotik_common::info_with_label;
use tokio::time::Instant;
use tokio::time::sleep;
use xshell::Shell;

mod catalog;
mod chr;
mod error;
mod manifest;
mod mikrotikd;
mod qemu;
mod scenario;

pub use catalog::ChrArch;
pub use catalog::DEFAULT_ROUTEROS_VERSION;
pub use catalog::ROUTEROS_VERSIONS;
pub use catalog::RouterOsChannel;
pub use catalog::RouterOsVersion;
pub use chr::MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL;
pub use error::Error;
pub use error::Result;
pub use mikrotikd::CommandAttribute;
pub use mikrotikd::DEFAULT_CPUS;
pub use mikrotikd::DEFAULT_MEMORY_MIB;
pub use mikrotikd::DEFAULT_PASSWORD;
pub use mikrotikd::DEFAULT_USERNAME;
pub use mikrotikd::MikrotikD;
pub use mikrotikd::MikrotikDConf;
pub use mikrotikd::RouterCommand;
pub use scenario::EthernetInterface;
pub use scenario::EthernetLink;
pub use scenario::Scenario;
pub use scenario::ScenarioConf;

use crate::catalog::ChrArch as RuntimeArch;
use crate::chr::IMAGES_DIR;
use crate::chr::ensure_chr_image;
use crate::qemu::RuntimeTarget;
use crate::qemu::create_overlay;
use crate::qemu::ensure_tool;
use crate::qemu::qemu_system_binary;

/// Root directory for cached CHR images and local runner runtime state.
const CACHE_DIR: &str = ".chr-cache";
/// Directory for per-invocation overlays, sockets, logs, and pid files.
const RUNS_DIR: &str = ".chr-cache/runs";
/// Maximum time to wait for API login after starting devices.
pub(crate) const DEFAULT_BOOT_TIMEOUT: Duration = Duration::from_secs(600);
/// Per-router serial console log filename suffix.
pub(crate) const SERIAL_LOG_SUFFIX: &str = ".serial.log";
/// Per-router QEMU command-line artifact filename suffix.
pub(crate) const QEMU_ARGS_SUFFIX: &str = ".qemu.args";
/// Per-router QEMU stderr log filename suffix.
pub(crate) const QEMU_LOG_SUFFIX: &str = ".qemu.log";
/// Per-router QEMU process id filename suffix.
pub(crate) const PID_FILE_SUFFIX: &str = ".pid";
/// Scenario summary report filename.
const SCENARIO_REPORT_FILENAME: &str = "scenario.csv";

/// Devices spawned together from configuration.
pub(crate) struct SpawnedMikrotiks {
    /// Per-run artifact directory.
    pub(crate) run_dir: PathBuf,
    /// Running devices.
    pub(crate) devices: Vec<MikrotikD>,
    /// Runtime socket directory removed after routers are dropped.
    pub(crate) socket_dir_guard: RuntimeSocketDir,
}

/// Router runtime inputs prepared before QEMU is spawned.
pub(crate) struct PreparedRouter {
    /// Router index in configuration order.
    pub(crate) index: usize,
    /// Router configuration.
    pub(crate) config: MikrotikDConf,
    /// Host API port assigned to this router.
    pub(crate) api_port: u16,
    /// Runtime target selected for this router.
    pub(crate) target: RuntimeTarget,
    /// QEMU system binary selected for this router.
    pub(crate) qemu_system: String,
    /// Per-run overlay image.
    pub(crate) overlay: PathBuf,
}

/// Inputs shared while spawning routers for one scenario run.
pub(crate) struct StartContext<'a> {
    /// QEMU system binary selected for the host architecture.
    pub(crate) qemu_system: &'a str,
    /// Runtime target selected for this router.
    pub(crate) target: RuntimeTarget,
    /// Per-run directory for overlays, logs, sockets, and firmware vars.
    pub(crate) run_dir: &'a Path,
    /// Short temporary directory for QEMU Unix sockets.
    pub(crate) socket_dir: &'a Path,
    /// Scenario links used to reserve hotpluggable `PCIe` slots.
    pub(crate) links: &'a [EthernetLink],
    /// Shell used to construct host commands.
    pub(crate) sh: &'a Shell,
}

/// Temporary QEMU socket directory removed after a simulation run.
#[derive(Debug)]
pub(crate) struct RuntimeSocketDir(PathBuf);

impl Drop for RuntimeSocketDir {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            error_with_label!(
                "QEMU",
                "Failed to remove runtime socket directory {}: {error}",
                self.0.display()
            );
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
            .ok_or_else(|| Error::Config(format!("missing API port allocation for `{router_name}`")))?;
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

/// Spawn a set of devices from config and wait for API readiness.
pub(crate) async fn spawn_mikrotikds(
    name: &str,
    allow_software_emulation: bool,
    routers_config: &[MikrotikDConf],
    links: &[EthernetLink],
) -> Result<SpawnedMikrotiks> {
    let config = ScenarioConf {
        name: name.to_owned(),
        allow_software_emulation,
        devices: routers_config.to_vec(),
        links: links.to_vec(),
    };
    validate_scenario(&config)?;

    let sh = Shell::new()?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    prepare_state_dirs(&sh, &root)?;
    ensure_tool(&sh, "qemu-img")?;

    let host_arch = RuntimeArch::host()?;
    let run_dir = run_dir(&sh, &root, &config.name)?;
    let socket_dir = socket_dir(&sh, &run_dir)?;
    let socket_dir_guard = RuntimeSocketDir(socket_dir.clone());
    let mut api_ports = allocate_api_ports(&config)?;
    let mut prepared = Vec::new();

    for (index, router) in config.devices.iter().enumerate() {
        let api_port = api_ports.release(&router.name)?;
        let version = router.version;
        let target = RuntimeTarget::detect(&sh, host_arch, version, config.allow_software_emulation)?;
        let qemu_system = qemu_system_binary(&sh, target.guest_arch)?;
        let base_image = ensure_chr_image(&root, version.as_str(), target.guest_arch)?;
        let overlay = run_dir.join(format!("{}.qcow2", router.name));
        debug_with_label!(router.name, "Creating overlay at {}", overlay.display());
        create_overlay(&sh, &base_image, &overlay)?;
        prepared.push(PreparedRouter {
            index,
            config: router.clone(),
            api_port,
            target,
            qemu_system,
            overlay,
        });
    }
    write_scenario_report(&run_dir, &config, &prepared)?;

    let mut devices = Vec::new();
    for router in &prepared {
        let start_context = StartContext {
            qemu_system: &router.qemu_system,
            target: router.target,
            run_dir: &run_dir,
            socket_dir: &socket_dir,
            links: &config.links,
            sh: &sh,
        };
        devices.push(MikrotikD::start(router, &start_context)?);
    }

    let mut clients = BTreeMap::new();
    for router in &devices {
        let client = MikrotikD::wait_for_client(router.name(), router.api_socket().port()).await?;
        clients.insert(router.name().to_owned(), client);
    }
    if !config.links.is_empty() {
        wait_for_link_interfaces(&config.links, &clients).await?;
    }
    bootstrap(&config.devices, &clients).await?;

    Ok(SpawnedMikrotiks {
        run_dir,
        devices,
        socket_dir_guard,
    })
}

/// Validate scenario config before touching runtime state.
fn validate_scenario(config: &ScenarioConf) -> Result<()> {
    if config.name.trim().is_empty() {
        return Err(Error::Config("scenario name cannot be empty".to_owned()));
    }
    if config.devices.is_empty() {
        return Err(Error::Config("scenario must contain at least one device".to_owned()));
    }

    let mut names = BTreeSet::new();
    for router in &config.devices {
        validate_router(router)?;
        if !names.insert(router.name.clone()) {
            return Err(Error::Config(format!("duplicate router `{}`", router.name)));
        }
    }
    for link in &config.links {
        validate_link_endpoint(&link.a)?;
        validate_link_endpoint(&link.b)?;
        if !names.contains(&link.a.router) {
            return Err(Error::Config(format!(
                "link references unknown router `{}`",
                link.a.router
            )));
        }
        if !names.contains(&link.b.router) {
            return Err(Error::Config(format!(
                "link references unknown router `{}`",
                link.b.router
            )));
        }
        if link.a.router == link.b.router {
            return Err(Error::Config(format!(
                "link cannot connect router `{}` to itself",
                link.a.router
            )));
        }
    }
    Ok(())
}

/// Validate one router config.
fn validate_router(router: &MikrotikDConf) -> Result<()> {
    if router.name.trim().is_empty() {
        return Err(Error::Config("router name cannot be empty".to_owned()));
    }
    if router.memory_mib == 0 {
        return Err(Error::Config(format!(
            "router `{}` memory_mib cannot be zero",
            router.name
        )));
    }
    if router.cpus == 0 {
        return Err(Error::Config(format!("router `{}` cpus cannot be zero", router.name)));
    }
    for command in &router.bootstrap {
        if command.command.trim().is_empty() {
            return Err(Error::Config(format!(
                "router `{}` contains an empty bootstrap command",
                router.name
            )));
        }
    }
    Ok(())
}

/// Validate one link endpoint.
fn validate_link_endpoint(endpoint: &scenario::EthernetEndpoint) -> Result<()> {
    if endpoint.router.trim().is_empty() {
        return Err(Error::Config("link endpoint router cannot be empty".to_owned()));
    }
    MikrotikD::link_interface_index(endpoint);
    Ok(())
}

/// Create persistent image and per-run state directories.
fn prepare_state_dirs(sh: &Shell, root: &Path) -> Result<()> {
    sh.create_dir(root.join(CACHE_DIR))?;
    sh.create_dir(root.join(IMAGES_DIR))?;
    sh.create_dir(root.join(RUNS_DIR))?;
    Ok(())
}

/// Create and return a unique run directory for this scenario invocation.
fn run_dir(sh: &Shell, root: &Path, scenario_name: &str) -> Result<PathBuf> {
    for _ in 0..3 {
        let timestamp = sh
            .cmd("date")
            .arg("+%Y%m%d-%H%M%S")
            .read()
            .map_err(|error| Error::Tool(format!("format local timestamp with `date`: {error}")))?;
        let path = root.join(RUNS_DIR).join(format!("{timestamp}-{scenario_name}"));
        if !path.exists() {
            sh.create_dir(&path)?;
            return Ok(path);
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err(Error::Tool(format!(
        "could not create a unique timestamped run directory for scenario `{scenario_name}`"
    )))
}

/// Create a short path for QEMU Unix sockets.
fn socket_dir(sh: &Shell, run_dir: &Path) -> Result<PathBuf> {
    let run_name = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Tool(format!("read run directory name from {}", run_dir.display())))?;
    let path = PathBuf::from("/tmp").join(format!("mikrotik-qemu-runner-{run_name}"));
    if path.exists() {
        fs::remove_dir_all(&path)?;
    }
    sh.create_dir(&path)?;
    Ok(path)
}

/// Wait until every link endpoint named by the scenario exists in `RouterOS`.
async fn wait_for_link_interfaces(links: &[EthernetLink], clients: &BTreeMap<String, Client>) -> Result<()> {
    let mut expected = BTreeMap::<String, Vec<EthernetInterface>>::new();
    for link in links {
        expected
            .entry(link.a.router.clone())
            .or_default()
            .push(link.a.interface);
        expected
            .entry(link.b.router.clone())
            .or_default()
            .push(link.b.interface);
    }

    let deadline = Instant::now() + DEFAULT_BOOT_TIMEOUT;
    loop {
        let mut missing = Vec::new();
        for (router, interfaces) in &expected {
            let client = clients
                .get(router)
                .ok_or_else(|| Error::Config(format!("missing connected client for `{router}`")))?;
            let rows = client.call("/interface/print", &[]).await?;
            for interface in interfaces {
                let interface_name = interface.to_string();
                if !rows.iter().any(|row| row.get("name") == Some(&interface_name)) {
                    missing.push(format!("{router}:{interface}"));
                }
            }
        }

        if missing.is_empty() {
            info_with_label!("Scenario", "All hotplugged link interfaces are visible");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::Tool(format!(
                "hotplugged link interface(s) did not appear: {}",
                missing.join(", ")
            )));
        }

        info_with_label!(
            "Scenario",
            "Waiting for hotplugged interface(s): {}",
            missing.join(", ")
        );
        sleep(Duration::from_secs(2)).await;
    }
}

/// Apply configured bootstrap commands to every router.
async fn bootstrap(routers: &[MikrotikDConf], clients: &BTreeMap<String, Client>) -> Result<()> {
    for router in routers {
        let client = clients
            .get(&router.name)
            .ok_or_else(|| Error::Config(format!("missing connected client for `{}`", router.name)))?;
        info_with_label!(router.name, "Applying {} bootstrap command(s)", router.bootstrap.len());
        for command in &router.bootstrap {
            info_with_label!(router.name, "Bootstrap {}", command.command);
            let attributes = command
                .attributes
                .iter()
                .map(|attribute| (attribute.key.as_str(), attribute.value.as_deref()))
                .collect::<Vec<_>>();
            client.call(&command.command, &attributes).await.map_err(|error| {
                Error::Tool(format!(
                    "router {} bootstrap command {}: {error}",
                    router.name, command.command
                ))
            })?;
        }
    }
    Ok(())
}

/// Write the planned scenario/runtime target report before QEMU starts.
fn write_scenario_report(run_dir: &Path, config: &ScenarioConf, routers: &[PreparedRouter]) -> Result<()> {
    let mut lines = vec![
        "scenario,routers,links,index,router,version,host_arch,guest_arch,accelerator,qemu_system,api_port,bootstrap_commands".to_owned(),
    ];

    for router in routers {
        lines.push(
            [
                csv_field(&config.name),
                config.devices.len().to_string(),
                config.links.len().to_string(),
                router.index.to_string(),
                csv_field(&router.config.name),
                csv_field(router.config.version.as_str()),
                format!("{:?}", router.target.host_arch()),
                format!("{:?}", router.target.guest_arch),
                csv_field(router.target.accelerator_name()),
                csv_field(&router.qemu_system),
                router.api_port.to_string(),
                router.config.bootstrap.len().to_string(),
            ]
            .join(","),
        );
    }

    fs::write(
        run_dir.join(SCENARIO_REPORT_FILENAME),
        format!("{}\n", lines.join("\n")),
    )?;
    Ok(())
}

/// Allocate and reserve host API ports for all routers.
fn allocate_api_ports(scenario: &ScenarioConf) -> Result<ApiPortAllocations> {
    let mut ports = BTreeMap::new();

    for router in &scenario.devices {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| Error::Tool(format!("reserve dynamic API port for {}: {error}", router.name)))?;
        let port = listener
            .local_addr()
            .map_err(|error| Error::Tool(format!("read reserved API port for {}: {error}", router.name)))?
            .port();
        info_with_label!(
            router.name,
            "API localhost:{port} Username={} Password={}",
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
const fn display_password(password: &str) -> &str {
    if password.is_empty() { "<empty>" } else { password }
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
    use super::*;

    #[test]
    fn api_port_allocation_produces_unique_localhost_sockets() {
        let config = ScenarioConf::new("ports")
            .with_device(&MikrotikDConf::new("R01"))
            .with_device(&MikrotikDConf::new("R02"));

        let allocations = allocate_api_ports(&config).unwrap();
        let r01 = allocations.ports.get("R01").unwrap().port;
        let r02 = allocations.ports.get("R02").unwrap().port;

        assert_ne!(r01, r02);
        assert_ne!(r01, 0);
        assert_ne!(r02, 0);
    }

    #[test]
    fn config_validation_catches_bad_link_endpoints() {
        let config = ScenarioConf::new("bad")
            .with_device(&MikrotikDConf::new("R01"))
            .with_ethernet_link(EthernetLink {
                a: scenario::EthernetEndpoint {
                    router: "R01".to_owned(),
                    interface: EthernetInterface::new(2).unwrap(),
                },
                b: scenario::EthernetEndpoint {
                    router: "missing".to_owned(),
                    interface: EthernetInterface::new(2).unwrap(),
                },
            });

        assert!(matches!(validate_scenario(&config), Err(Error::Config(_))));
    }

    #[test]
    fn mikrotikd_config_defaults_match_runner_defaults() {
        let config = MikrotikDConf::new("R01");

        assert_eq!(config.name, "R01");
        assert_eq!(config.version, DEFAULT_ROUTEROS_VERSION);
        assert_eq!(config.memory_mib, DEFAULT_MEMORY_MIB);
        assert_eq!(config.cpus, DEFAULT_CPUS);
        assert!(config.allow_software_emulation);
        assert!(config.bootstrap.is_empty());
    }

    #[test]
    fn routeros_version_catalog_matches_runner_catalog_order() {
        let versions = ROUTEROS_VERSIONS
            .iter()
            .map(|version| version.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            versions,
            [
                "7.23.1", "7.23", "7.22.3", "7.22.2", "7.22.1", "7.21.4", "7.20.8", "7.20.7", "6.49.19", "6.49.18",
                "6.49.17", "6.49.16", "6.49.15", "6.49.13", "6.49.10",
            ]
        );
    }

    #[test]
    fn target_for_port_uses_default_chr_credentials() {
        let target = MikrotikD::target_for_port(18_728).unwrap();

        assert_eq!(target.address.to_string(), "127.0.0.1:18728");
        assert_eq!(target.credentials.username, DEFAULT_USERNAME);
        assert_eq!(target.credentials.password.as_deref(), Some(DEFAULT_PASSWORD));
    }
}
