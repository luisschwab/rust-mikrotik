//! Scenario configuration and lifecycle.

use core::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use mikrotik_common::info_with_label;
use mikrotik_types::target::DeviceTarget;

use crate::Error;
use crate::MikrotikD;
use crate::MikrotikDConf;
use crate::Result;
use crate::RuntimeSocketDir;
use crate::manifest;
use crate::spawn_mikrotikds;

/// One `RouterOS` Ethernet interface addressed by `RouterOS`' `etherN` naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EthernetInterface {
    /// One-based `etherN` interface index.
    index: usize,
}

impl EthernetInterface {
    /// Build an Ethernet interface from a one-based `RouterOS` `etherN` index.
    ///
    /// # Errors
    ///
    /// Returns an error if `index` is zero.
    pub fn new(index: usize) -> Result<Self> {
        if index == 0 {
            return Err(Error::Config("Ethernet interface index cannot be zero".to_owned()));
        }
        Ok(Self { index })
    }

    /// Return the one-based `etherN` index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for EthernetInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ether{}", self.index)
    }
}

impl FromStr for EthernetInterface {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let index = value
            .strip_prefix("ether")
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or_else(|| Error::Config(format!("interface `{value}` must be named etherN")))?;
        Self::new(index)
    }
}

/// Configuration for a point-to-point Ethernet link between two devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EthernetLink {
    /// First endpoint.
    pub(crate) a: EthernetEndpoint,
    /// Second endpoint.
    pub(crate) b: EthernetEndpoint,
}

impl EthernetLink {
    /// Build a point-to-point Ethernet link between two configured `MikrotikD`s.
    #[must_use]
    pub fn create(
        a_device: &MikrotikDConf,
        a_interface: EthernetInterface,
        b_device: &MikrotikDConf,
        b_interface: EthernetInterface,
    ) -> Self {
        Self {
            a: EthernetEndpoint {
                router: a_device.name.clone(),
                interface: a_interface,
            },
            b: EthernetEndpoint {
                router: b_device.name.clone(),
                interface: b_interface,
            },
        }
    }
}

/// One Ethernet link endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EthernetEndpoint {
    /// Router name.
    pub(crate) router: String,
    /// Interface name.
    pub(crate) interface: EthernetInterface,
}

/// Configuration for a code-built scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioConf {
    /// Human-readable scenario name.
    pub name: String,
    /// Allow software QEMU emulation when hardware acceleration is unavailable.
    pub allow_software_emulation: bool,
    /// Devices in deterministic startup order.
    pub devices: Vec<MikrotikDConf>,
    /// Point-to-point Ethernet links.
    pub links: Vec<EthernetLink>,
}

impl ScenarioConf {
    /// Build an empty scenario config.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            allow_software_emulation: true,
            devices: Vec::new(),
            links: Vec::new(),
        }
    }

    /// Read a scenario config from a TOML scenario manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or the manifest is invalid.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        manifest::read_scenario_conf(path.as_ref())
    }

    /// Parse a scenario config from a TOML scenario manifest string.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest is invalid.
    pub fn parse(contents: &str) -> Result<Self> {
        manifest::parse_scenario_conf(contents)
    }

    /// Allow or reject software QEMU emulation.
    #[must_use]
    pub const fn with_software_emulation(mut self, allow: bool) -> Self {
        self.allow_software_emulation = allow;
        self
    }

    /// Add one `MikrotikD` config.
    #[must_use]
    pub fn with_device(mut self, device: &MikrotikDConf) -> Self {
        self.devices.push(device.clone());
        self
    }

    /// Add a point-to-point Ethernet link.
    #[must_use]
    pub fn with_ethernet_link(mut self, link: EthernetLink) -> Self {
        self.links.push(link);
        self
    }

    /// Add a point-to-point Ethernet link.
    #[must_use]
    pub fn with_link(self, link: EthernetLink) -> Self {
        self.with_ethernet_link(link)
    }
}

impl Default for ScenarioConf {
    fn default() -> Self {
        Self::new("scenario").with_device(&MikrotikDConf::default())
    }
}

/// One live simulated scenario.
#[derive(Debug)]
pub struct Scenario {
    /// Scenario name.
    name: String,
    /// Per-run artifact directory.
    run_dir: PathBuf,
    /// Running devices in configuration order.
    devices: Vec<MikrotikD>,
    /// Runtime socket directory removed after devices are dropped.
    socket_dir_guard: RuntimeSocketDir,
}

impl Scenario {
    /// Spawn a scenario with a default [`ScenarioConf`] and wait for router API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or an API does not become ready.
    pub async fn new() -> Result<Self> {
        Self::new_with_conf(&ScenarioConf::default()).await
    }

    /// Spawn a scenario with a custom [`ScenarioConf`] and wait for router API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or an API does not become ready.
    pub async fn new_with_conf(config: &ScenarioConf) -> Result<Self> {
        Self::spawn(config).await
    }

    /// Spawn a code-built scenario and wait for router API readiness.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid, required host tools are
    /// unavailable, image preparation fails, QEMU cannot start, bootstrap
    /// fails, or an API does not become ready.
    pub async fn spawn(config: &ScenarioConf) -> Result<Self> {
        let spawned = spawn_mikrotikds(
            &config.name,
            config.allow_software_emulation,
            &config.devices,
            &config.links,
        )
        .await?;
        Ok(Self {
            name: config.name.clone(),
            run_dir: spawned.run_dir,
            devices: spawned.devices,
            socket_dir_guard: spawned.socket_dir_guard,
        })
    }

    /// Return the scenario name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the live devices in configuration order.
    #[must_use]
    pub fn devices(&self) -> &[MikrotikD] {
        &self.devices
    }

    /// Find a live device by name.
    #[must_use]
    pub fn device(&self, name: &str) -> Option<&MikrotikD> {
        self.devices.iter().find(|device| device.name() == name)
    }

    /// Return all crawler targets in configuration order.
    #[must_use]
    pub fn targets(&self) -> Vec<DeviceTarget> {
        self.devices.iter().map(|device| device.target().clone()).collect()
    }

    /// Return the per-run artifact directory.
    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    /// Stop and reap every device immediately.
    pub fn shutdown(&mut self) {
        if !self.devices.is_empty() {
            info_with_label!(self.name.as_str(), "Stopping {} device process(es)", self.devices.len());
        }
        for device in &mut self.devices {
            device.shutdown();
        }
    }
}

impl Drop for Scenario {
    fn drop(&mut self) {
        self.shutdown();
        let _ = &self.socket_dir_guard;
    }
}
