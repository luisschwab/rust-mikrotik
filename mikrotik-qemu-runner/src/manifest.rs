//! TOML scenario manifest loading.

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::Error;
use crate::EthernetInterface;
use crate::EthernetLink;
use crate::MikrotikDConf;
use crate::Result;
use crate::RouterCommand;
use crate::RouterOsVersion;
use crate::ScenarioConf;

/// Load a scenario config from a TOML scenario manifest.
pub(crate) fn read_scenario_conf(path: &Path) -> Result<ScenarioConf> {
    let contents = fs::read_to_string(path).map_err(|source| Error::io("read scenario manifest", path, source))?;
    match parse_scenario_conf(&contents) {
        Err(Error::Config(message)) => Err(Error::Config(format!(
            "scenario manifest {}: {message}",
            path.display()
        ))),
        result => result,
    }
}

/// Parse a scenario config from a TOML scenario manifest.
pub(crate) fn parse_scenario_conf(contents: &str) -> Result<ScenarioConf> {
    let manifest = toml::from_str::<ScenarioManifest>(contents)
        .map_err(|error| Error::Config(format!("invalid scenario TOML: {}", error.message())))?;
    manifest.try_into_scenario_conf()
}

/// TOML shape for one QEMU runner scenario.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioManifest {
    /// Human-readable scenario name.
    name: String,
    /// Allow software QEMU emulation when hardware acceleration is unavailable.
    #[serde(default = "default_allow_software_emulation")]
    allow_software_emulation: bool,
    /// Device manifests.
    #[serde(default, alias = "routers")]
    devices: Vec<DeviceManifest>,
    /// Link manifests.
    #[serde(default)]
    links: Vec<LinkManifest>,
}

impl ScenarioManifest {
    /// Convert the TOML shape into runner config.
    fn try_into_scenario_conf(self) -> Result<ScenarioConf> {
        let devices = self
            .devices
            .into_iter()
            .map(DeviceManifest::try_into_mikrotikd_conf)
            .collect::<Result<Vec<_>>>()?;

        let mut scenario = ScenarioConf::new(self.name).with_software_emulation(self.allow_software_emulation);
        for device in &devices {
            scenario = scenario.with_device(device);
        }
        for link in self.links {
            scenario = scenario.with_ethernet_link(link.try_into_ethernet_link(&devices)?);
        }

        Ok(scenario)
    }
}

/// TOML shape for one `MikroTik` device.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceManifest {
    /// Stable device name.
    name: String,
    /// `RouterOS` version used for the CHR image.
    #[serde(default)]
    version: RouterOsVersion,
    /// Memory in MiB.
    #[serde(default = "default_memory_mib")]
    memory_mib: u16,
    /// CPU count.
    #[serde(default = "default_cpus")]
    cpus: u8,
    /// Bootstrap commands.
    #[serde(default)]
    bootstrap: Vec<String>,
}

impl DeviceManifest {
    /// Convert the TOML shape into a device config.
    fn try_into_mikrotikd_conf(self) -> Result<MikrotikDConf> {
        let mut config = MikrotikDConf::new(self.name)
            .with_version(self.version)
            .with_memory_mib(self.memory_mib)
            .with_cpus(self.cpus);
        let bootstrap = self
            .bootstrap
            .iter()
            .map(|command| parse_router_command(command))
            .collect::<Result<Vec<_>>>()?;
        for command in bootstrap {
            config = config.with_bootstrap(command);
        }
        Ok(config)
    }
}

/// TOML shape for one point-to-point Ethernet link.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkManifest {
    /// First endpoint as `device:interface`.
    a: String,
    /// Second endpoint as `device:interface`.
    b: String,
}

impl LinkManifest {
    /// Convert the TOML shape into a typed Ethernet link.
    fn try_into_ethernet_link(self, devices: &[MikrotikDConf]) -> Result<EthernetLink> {
        let a = parse_endpoint(&self.a)?;
        let b = parse_endpoint(&self.b)?;
        let a_device = find_device(devices, a.device)?;
        let b_device = find_device(devices, b.device)?;
        Ok(EthernetLink::create(a_device, a.interface, b_device, b.interface))
    }
}

/// One parsed link endpoint.
#[derive(Debug, Clone, Copy)]
struct Endpoint<'a> {
    /// Device name.
    device: &'a str,
    /// Ethernet interface.
    interface: EthernetInterface,
}

/// Default device memory in MiB.
const fn default_memory_mib() -> u16 {
    crate::DEFAULT_MEMORY_MIB
}

/// Default device CPU count.
const fn default_cpus() -> u8 {
    crate::DEFAULT_CPUS
}

/// Default software-emulation policy.
const fn default_allow_software_emulation() -> bool {
    crate::DEFAULT_ALLOW_SOFTWARE_EMULATION
}

/// Parse a `device:interface` endpoint string.
fn parse_endpoint(value: &str) -> Result<Endpoint<'_>> {
    let (device, interface) = value
        .split_once(':')
        .ok_or_else(|| Error::Config(format!("endpoint `{value}` must use device:interface")))?;
    Ok(Endpoint {
        device,
        interface: interface.parse()?,
    })
}

/// Find a named device config.
fn find_device<'a>(devices: &'a [MikrotikDConf], name: &str) -> Result<&'a MikrotikDConf> {
    devices
        .iter()
        .find(|device| device.name == name)
        .ok_or_else(|| Error::Config(format!("unknown device `{name}`")))
}

/// Parse a bootstrap command string into command path and attributes.
fn parse_router_command(value: &str) -> Result<RouterCommand> {
    let mut parts = value.split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| Error::Config("bootstrap command must not be empty".to_owned()))?;
    if !command.starts_with('/') {
        return Err(Error::Config(format!(
            "bootstrap command `{command}` must start with `/`"
        )));
    }

    let mut parsed = RouterCommand::new(command);
    for part in parts {
        let (key, value) = part
            .split_once('=')
            .map_or_else(|| (part, None), |(key, value)| (key, Some(value)));
        if key.is_empty() {
            return Err(Error::Config(format!(
                "bootstrap command `{command}` has an empty attribute key"
            )));
        }
        parsed = match value {
            Some(value) => parsed.with_attribute(key, value),
            None => parsed.with_flag(key),
        };
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::process;

    use super::*;

    #[test]
    fn parses_scenario_manifest_into_scenario_conf() {
        let scenario = parse_scenario_conf(include_str!("../scenarios/two-router.toml")).unwrap();

        assert_eq!(scenario.name, "two-router");
        assert!(scenario.allow_software_emulation);
        assert_eq!(scenario.devices.len(), 2);
        assert_eq!(scenario.devices[0].name, "R01");
        assert_eq!(scenario.devices[0].bootstrap[1].command, "/ip/address/add");
        assert_eq!(scenario.devices[0].bootstrap[1].attributes[1].key, "interface");
        assert_eq!(
            scenario.devices[0].bootstrap[1].attributes[1].value.as_deref(),
            Some("ether2")
        );
        assert_eq!(scenario.links.len(), 1);
    }

    #[test]
    fn manifest_uses_code_configuration_defaults() {
        let scenario = parse_scenario_conf(
            r#"
name = "defaults"

[[devices]]
name = "R01"
"#,
        )
        .expect("minimal scenario should parse");

        assert!(scenario.allow_software_emulation);
        assert_eq!(scenario.devices[0].version, crate::DEFAULT_ROUTEROS_VERSION);
        assert_eq!(scenario.devices[0].memory_mib, crate::DEFAULT_MEMORY_MIB);
        assert_eq!(scenario.devices[0].cpus, crate::DEFAULT_CPUS);
    }

    #[test]
    fn rejects_unknown_routeros_versions() {
        let error = parse_scenario_conf(
            r#"
name = "bad"

[[routers]]
name = "R01"
version = "1.2.3"
"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown RouterOS version"));
    }

    #[test]
    fn reads_manifests_and_adds_the_source_path_to_configuration_errors() {
        let path = std::env::temp_dir().join(format!("mikrotik-qemu-manifest-{}.toml", process::id()));
        fs::write(&path, "name = \"read-test\"\n[[devices]]\nname = \"R01\"\n").unwrap();
        assert_eq!(read_scenario_conf(&path).unwrap().name, "read-test");

        fs::write(&path, "name = \"bad\"\nunknown = true\n").unwrap();
        let error = read_scenario_conf(&path).unwrap_err();
        assert!(error.to_string().contains(&path.display().to_string()));
        fs::remove_file(&path).unwrap();

        let error = read_scenario_conf(&path).unwrap_err();
        assert!(error.to_string().contains("read scenario manifest"));
    }

    #[test]
    fn endpoint_and_bootstrap_parsers_reject_malformed_input() {
        let devices = [MikrotikDConf::new("R01")];
        assert!(parse_endpoint("R01").is_err());
        assert!(parse_endpoint("R01:not-an-interface").is_err());
        assert!(find_device(&devices, "missing").is_err());
        assert!(parse_router_command("").is_err());
        assert!(parse_router_command("system/identity/print").is_err());
        assert!(parse_router_command("/system/identity/set =value").is_err());

        let command = parse_router_command("/system/identity/set disabled name=router").unwrap();
        assert_eq!(command.command, "/system/identity/set");
        assert_eq!(command.attributes[0].key, "disabled");
        assert_eq!(command.attributes[0].value, None);
        assert_eq!(command.attributes[1].value.as_deref(), Some("router"));
    }

    #[test]
    fn manifests_reject_unknown_devices_and_invalid_endpoints() {
        for manifest in [
            "name = \"bad\"\n[[devices]]\nname = \"R01\"\n[[links]]\na = \"missing:ether1\"\nb = \"R01:ether1\"\n",
            "name = \"bad\"\n[[devices]]\nname = \"R01\"\n[[links]]\na = \"R01\"\nb = \"R01:ether1\"\n",
            "name = \"bad\"\n[[devices]]\nname = \"R01\"\nbootstrap = [\"invalid\"]\n",
        ] {
            assert!(parse_scenario_conf(manifest).is_err());
        }
    }
}
