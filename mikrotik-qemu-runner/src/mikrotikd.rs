//! Running `MikrotikD` device lifecycle.

use core::fmt;
use core::net::Ipv4Addr;
use core::net::SocketAddr;
use core::slice;
use core::time::Duration;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;

use mikrotik_client::builder::ClientBuilder;
use mikrotik_client::builder::Protocol;
use mikrotik_client::client::Client;
use mikrotik_common::info_with_label;
use mikrotik_common::redaction::is_sensitive_key;
use mikrotik_types::target::Credentials;
use mikrotik_types::target::DeviceTarget;
use tokio::time::Instant;

use crate::DEFAULT_ALLOW_SOFTWARE_EMULATION;
use crate::DEFAULT_BOOT_TIMEOUT;
use crate::DEFAULT_ROUTEROS_VERSION;
use crate::Error;
use crate::EthernetLink;
use crate::PID_FILE_SUFFIX;
use crate::PreparedRouter;
use crate::QEMU_ARGS_SUFFIX;
use crate::QEMU_LOG_SUFFIX;
use crate::Result;
use crate::RouterOsVersion;
use crate::SERIAL_LOG_SUFFIX;
use crate::StartContext;
use crate::qemu::QemuVm;
use crate::qemu::append_accelerator_args;
use crate::qemu::append_disk_args;
#[cfg(test)]
use crate::qemu::target_for_port;
use crate::scenario::EthernetEndpoint;
use crate::spawn_mikrotikds;

/// Default CHR memory allocation in MiB.
pub const DEFAULT_MEMORY_MIB: u16 = 256;
/// Default CHR virtual CPU count.
pub const DEFAULT_CPUS: u8 = 1;
/// Default CHR admin username.
pub const DEFAULT_USERNAME: &str = "admin";
/// Default CHR admin password before bootstrap.
pub const DEFAULT_PASSWORD: &str = "";

/// Configuration for one CHR router VM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MikrotikDConf {
    /// Stable router name.
    pub name: String,
    /// `RouterOS` version used for the CHR image.
    pub version: RouterOsVersion,
    /// Memory in MiB.
    pub memory_mib: u16,
    /// CPU count.
    pub cpus: u8,
    /// Allow software QEMU emulation when hardware acceleration is unavailable.
    pub allow_software_emulation: bool,
    /// Commands applied after first API login.
    pub bootstrap: Vec<RouterCommand>,
}

impl MikrotikDConf {
    /// Build a router config with QEMU runner defaults.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: DEFAULT_ROUTEROS_VERSION,
            memory_mib: DEFAULT_MEMORY_MIB,
            cpus: DEFAULT_CPUS,
            allow_software_emulation: DEFAULT_ALLOW_SOFTWARE_EMULATION,
            bootstrap: Vec::new(),
        }
    }

    /// Override the `RouterOS` version.
    #[must_use]
    pub const fn with_version(mut self, version: RouterOsVersion) -> Self {
        self.version = version;
        self
    }

    /// Override guest memory in MiB.
    #[must_use]
    pub const fn with_memory_mib(mut self, memory_mib: u16) -> Self {
        self.memory_mib = memory_mib;
        self
    }

    /// Override the guest virtual CPU count.
    #[must_use]
    pub const fn with_cpus(mut self, cpus: u8) -> Self {
        self.cpus = cpus;
        self
    }

    /// Allow or reject software QEMU emulation for single-VM spawns.
    #[must_use]
    pub const fn with_software_emulation(mut self, allow: bool) -> Self {
        self.allow_software_emulation = allow;
        self
    }

    /// Add one bootstrap command.
    #[must_use]
    pub fn with_bootstrap(mut self, command: RouterCommand) -> Self {
        self.bootstrap.push(command);
        self
    }
}

impl Default for MikrotikDConf {
    fn default() -> Self {
        Self::new("router")
    }
}

/// Raw `RouterOS` command with optional attributes.
#[derive(Clone, PartialEq, Eq)]
pub struct RouterCommand {
    /// `RouterOS` API command path.
    pub command: String,
    /// Command attributes in call order.
    pub attributes: Vec<CommandAttribute>,
}

impl fmt::Debug for RouterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouterCommand")
            .field("command", &self.command)
            .field(
                "attributes",
                &RedactedCommandAttributes {
                    command: &self.command,
                    attributes: &self.attributes,
                },
            )
            .finish()
    }
}

impl RouterCommand {
    /// Build a raw `RouterOS` command.
    #[must_use]
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            attributes: Vec::new(),
        }
    }

    /// Add a key/value attribute.
    #[must_use]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(CommandAttribute {
            key: key.into(),
            value: Some(value.into()),
        });
        self
    }

    /// Add a flag attribute.
    #[must_use]
    pub fn with_flag(mut self, key: impl Into<String>) -> Self {
        self.attributes.push(CommandAttribute {
            key: key.into(),
            value: None,
        });
        self
    }
}

/// One `RouterOS` command attribute.
#[derive(Clone, PartialEq, Eq)]
pub struct CommandAttribute {
    /// Attribute key without leading `=`.
    pub key: String,
    /// Attribute value, or `None` for a flag attribute.
    pub value: Option<String>,
}

impl fmt::Debug for CommandAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        RedactedCommandAttribute {
            command: "",
            attribute: self,
        }
        .fmt(f)
    }
}

/// Debug wrapper that applies command-aware redaction to an attribute list.
struct RedactedCommandAttributes<'a> {
    /// `RouterOS` command path.
    command: &'a str,
    /// Command attributes in call order.
    attributes: &'a [CommandAttribute],
}

impl fmt::Debug for RedactedCommandAttributes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for attribute in self.attributes {
            list.entry(&RedactedCommandAttribute {
                command: self.command,
                attribute,
            });
        }
        list.finish()
    }
}

/// Debug wrapper for one potentially sensitive command attribute.
struct RedactedCommandAttribute<'a> {
    /// `RouterOS` command path.
    command: &'a str,
    /// Attribute being formatted.
    attribute: &'a CommandAttribute,
}

impl fmt::Debug for RedactedCommandAttribute<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let redact = is_sensitive_key(&self.attribute.key)
            || (self.command.starts_with("/snmp/community/") && self.attribute.key == "name");
        let value = if redact && self.attribute.value.is_some() {
            Some("<redacted>")
        } else {
            self.attribute.value.as_deref()
        };
        f.debug_struct("CommandAttribute")
            .field("key", &self.attribute.key)
            .field("value", &value)
            .finish()
    }
}

/// One live simulated router VM.
#[derive(Debug)]
pub struct MikrotikD {
    /// Router config used to start this VM.
    pub(crate) config: MikrotikDConf,
    /// Inner running QEMU VM.
    pub(crate) inner: QemuVm,
}

impl MikrotikD {
    /// Spawn a single router VM with the default [`MikrotikDConf`] and wait for API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or the API does not become ready.
    pub async fn new() -> Result<Self> {
        Self::new_with_conf(&MikrotikDConf::default()).await
    }

    /// Spawn a single router VM with a custom [`MikrotikDConf`] and wait for API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or the API does not become ready.
    pub async fn new_with_conf(config: &MikrotikDConf) -> Result<Self> {
        Self::spawn(config).await
    }

    /// Spawn a single router VM and wait for API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or the API does not become ready.
    pub async fn spawn(config: &MikrotikDConf) -> Result<Self> {
        let crate::SpawnedMikrotiks {
            mut devices,
            socket_dir_guard,
            ..
        } = spawn_mikrotikds(
            &config.name,
            config.allow_software_emulation,
            slice::from_ref(config),
            &[],
        )
        .await?;
        let mut device = devices
            .pop()
            .ok_or_else(|| Error::Tool("single-device spawn returned no device".to_owned()))?;
        device.inner.set_socket_dir_guard(socket_dir_guard);
        Ok(device)
    }

    /// Spawn one QEMU process for a router using the prepared overlay image.
    pub(crate) fn start(router: &PreparedRouter, context: &StartContext<'_>) -> Result<Self> {
        let router_name = &router.config.name;
        let serial_log_path = router_artifact_path(context.run_dir, router_name, SERIAL_LOG_SUFFIX);
        let qemu_args_path = router_artifact_path(context.run_dir, router_name, QEMU_ARGS_SUFFIX);
        let qemu_log_path = router_artifact_path(context.run_dir, router_name, QEMU_LOG_SUFFIX);
        let pid_file_path = router_artifact_path(context.run_dir, router_name, PID_FILE_SUFFIX);
        let mut args = vec![
            "-name".to_owned(),
            router_name.clone(),
            "-m".to_owned(),
            router.config.memory_mib.to_string(),
            "-smp".to_owned(),
            router.config.cpus.to_string(),
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

        let mgmt_network = Self::management_network(router.index)?;
        let hostfwd = format!("hostfwd=tcp:127.0.0.1:{}-:8728", router.api_port);
        args.extend([
            "-netdev".to_owned(),
            format!(
                "user,id=mgmt,net={}/24,dhcpstart={},{}",
                mgmt_network.network, mgmt_network.dhcp_start, hostfwd
            ),
            "-device".to_owned(),
            format!("virtio-net-pci,netdev=mgmt,mac={},addr=0x2", Self::mac(router.index, 0)),
        ]);
        for (link_index, link) in context.links.iter().enumerate() {
            if link.a.router == *router_name || link.b.router == *router_name {
                Self::append_link_args(&mut args, router, context, link_index, link);
            }
        }

        fs::write(&qemu_args_path, format!("{} {}\n", context.qemu_system, args.join(" ")))
            .map_err(|source| Error::io("write QEMU argument artifact", &qemu_args_path, source))?;

        info_with_label!(
            router_name,
            "Starting with {} on API localhost:{}",
            context.qemu_system,
            router.api_port
        );
        let mut command = context.sh.cmd(context.qemu_system).args(&args).to_command();
        let qemu_log =
            fs::File::create(&qemu_log_path).map_err(|source| Error::io("create QEMU log", &qemu_log_path, source))?;
        command.stdout(Stdio::null()).stderr(Stdio::from(qemu_log));

        let child = command
            .spawn()
            .map_err(|error| Error::Tool(format!("start {router_name} with {}: {error}", context.qemu_system)))?;
        let inner = QemuVm::new(
            router.config.name.clone(),
            router.api_port,
            context.run_dir.to_owned(),
            child,
        )?;

        Ok(Self {
            config: router.config.clone(),
            inner,
        })
    }

    /// Return the router name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Return the host socket forwarded to the `RouterOS` API service.
    #[must_use]
    pub const fn api_socket(&self) -> SocketAddr {
        self.inner.api_socket()
    }

    /// Return a client target for this simulated router.
    #[must_use]
    pub fn target(&self) -> &DeviceTarget {
        self.inner.target()
    }

    /// Open a fresh `RouterOS` API client for this router.
    ///
    /// # Errors
    ///
    /// Returns an error if the client cannot connect or authenticate.
    pub async fn client(&self) -> Result<Client> {
        Ok(Client::connect(Self::client_config(self.name(), self.api_socket().port())).await?)
    }

    /// Return the per-run artifact directory.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        self.inner.run_dir()
    }

    /// Stop and reap this router immediately.
    pub fn shutdown(&mut self) {
        self.inner.shutdown();
    }

    /// Build a crawler target for a forwarded localhost API port.
    #[cfg(test)]
    pub(crate) fn target_for_port(api_port: u16) -> Result<DeviceTarget> {
        target_for_port(api_port)
    }

    /// Wait until one router accepts API login and return a connected client.
    pub(crate) async fn wait_for_client(router_name: &str, api_port: u16) -> Result<Client> {
        let config = Self::client_config(router_name, api_port);

        let start = Instant::now();
        info_with_label!(
            router_name,
            "Waiting for API readiness at localhost:{api_port}, this may take a while..."
        );

        let client = Client::connect(config).await.map_err(|error| {
            Error::Tool(format!(
                "router {router_name} did not accept API login on localhost:{api_port}: {error}"
            ))
        })?;

        let elapsed = start.elapsed().as_secs();
        info_with_label!(router_name, "API ready on localhost:{api_port} after {elapsed} seconds");
        Ok(client)
    }

    /// Convert a linked `etherN` endpoint into a deterministic NIC index.
    pub(crate) const fn link_interface_index(endpoint: &EthernetEndpoint) -> usize {
        endpoint.interface.index()
    }

    /// Build a localhost API client configuration for a forwarded router port.
    fn client_config(router_name: &str, api_port: u16) -> ClientBuilder {
        ClientBuilder::new(
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

    /// Return a stable per-router management subnet for QEMU user networking.
    fn management_network(router_index: usize) -> Result<ManagementNetwork> {
        let subnet = u8::try_from(router_index + 1)
            .map_err(|_| Error::Config("too many routers for management subnet allocation".to_owned()))?;
        if subnet == 255 {
            return Err(Error::Config(
                "too many routers for management subnet allocation".to_owned(),
            ));
        }

        Ok(ManagementNetwork {
            network: Ipv4Addr::new(10, 64, subnet, 0),
            dhcp_start: Ipv4Addr::new(10, 64, subnet, 100),
        })
    }

    /// Append QEMU arguments for one point-to-point link NIC.
    fn append_link_args(
        args: &mut Vec<String>,
        router: &PreparedRouter,
        context: &StartContext<'_>,
        link_index: usize,
        link: &EthernetLink,
    ) {
        let router_name = &router.config.name;
        let endpoint = if link.a.router == *router_name {
            &link.a
        } else {
            &link.b
        };
        let nic_index = Self::link_interface_index(endpoint);
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
            crate::catalog::ChrArch::X86_64 => args.extend([
                "-device".to_owned(),
                format!(
                    "virtio-net-pci,netdev={netdev_id},mac={},addr=0x{:x}",
                    Self::mac(router.index, nic_index),
                    nic_index + 2
                ),
            ]),
            crate::catalog::ChrArch::Aarch64 => args.extend([
                "-device".to_owned(),
                format!(
                    "pcie-root-port,id={},chassis={},slot={},addr=0x{:x}",
                    Self::link_bus_id(link_index),
                    link_index + 1,
                    nic_index + 2,
                    nic_index + 2
                ),
                "-device".to_owned(),
                format!(
                    "virtio-net-pci,bus={},netdev={netdev_id},mac={}",
                    Self::link_bus_id(link_index),
                    Self::mac(router.index, nic_index),
                ),
            ]),
        }
    }

    /// Derive a deterministic locally administered MAC address.
    fn mac(router_index: usize, nic_index: usize) -> String {
        format!(
            "02:52:{:02x}:{:02x}:{:02x}:{:02x}",
            (router_index >> 8) & 0xff,
            router_index & 0xff,
            (nic_index >> 8) & 0xff,
            nic_index & 0xff
        )
    }

    /// QEMU bus ID reserved for one scenario link endpoint.
    fn link_bus_id(link_index: usize) -> String {
        format!("link{link_index}bus")
    }
}

/// One isolated QEMU user-mode management network.
struct ManagementNetwork {
    /// Network address assigned to QEMU SLIRP.
    network: Ipv4Addr,
    /// First DHCP lease address used before persistent management config exists.
    dhcp_start: Ipv4Addr,
}

/// Return the path for one per-router run artifact.
fn router_artifact_path(run_dir: &Path, router_name: &str, suffix: &str) -> PathBuf {
    run_dir.join(format!("{router_name}{suffix}"))
}

impl Drop for MikrotikD {
    fn drop(&mut self) {
        info_with_label!(self.name(), "Dropping...");
        self.shutdown();
        info_with_label!(self.name(), "Dropped");
    }
}
