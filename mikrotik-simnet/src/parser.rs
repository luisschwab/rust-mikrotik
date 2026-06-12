//! Deterministic topology manifest parser.

use serde::Deserialize;

use crate::error::Error;
use crate::error::Result;
use crate::runner::link_interface_index;
use crate::topology::Check;
use crate::topology::CommandAttribute;
use crate::topology::Endpoint;
use crate::topology::Link;
use crate::topology::Router;
use crate::topology::RouterCommand;
use crate::topology::Topology;

/// Parse the supported TOML topology manifest into a topology.
pub(crate) fn parse_topology(contents: &str) -> Result<Topology> {
    let manifest = toml::from_str::<TopologyManifest>(contents)
        .map_err(|error| Error::Manifest(format!("invalid topology TOML: {error}")))?;
    let topology = manifest.try_into_topology()?;
    validate_topology(&topology)?;
    Ok(topology)
}

/// TOML shape for a topology manifest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TopologyManifest {
    /// Human-readable topology name.
    name: String,
    /// Allow software QEMU emulation when hardware acceleration is unavailable.
    #[serde(default)]
    allow_software_emulation: bool,
    /// Router manifests.
    #[serde(default)]
    routers: Vec<RouterManifest>,
    /// Link manifests.
    #[serde(default)]
    links: Vec<LinkManifest>,
    /// Check manifests.
    #[serde(default)]
    checks: Vec<CheckManifest>,
}

impl TopologyManifest {
    /// Convert the TOML shape into the runtime topology shape.
    fn try_into_topology(self) -> Result<Topology> {
        Ok(Topology {
            name: self.name,
            allow_software_emulation: self.allow_software_emulation,
            routers: self
                .routers
                .into_iter()
                .map(RouterManifest::try_into_router)
                .collect::<Result<Vec<_>>>()?,
            links: self
                .links
                .into_iter()
                .map(LinkManifest::try_into_link)
                .collect::<Result<Vec<_>>>()?,
            checks: self
                .checks
                .into_iter()
                .map(CheckManifest::try_into_check)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

/// TOML shape for one router.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouterManifest {
    /// Stable router name.
    name: String,
    /// `RouterOS` version used for the CHR image.
    version: String,
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

impl RouterManifest {
    /// Convert the TOML shape into a router.
    fn try_into_router(self) -> Result<Router> {
        Ok(Router {
            name: self.name,
            version: self.version,
            memory_mib: self.memory_mib,
            cpus: self.cpus,
            bootstrap: self
                .bootstrap
                .iter()
                .map(|command| parse_router_command(command))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

/// TOML shape for one point-to-point link.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkManifest {
    /// First endpoint as `router:interface`.
    a: String,
    /// Second endpoint as `router:interface`.
    b: String,
}

impl LinkManifest {
    /// Convert the TOML shape into a link.
    fn try_into_link(self) -> Result<Link> {
        Ok(Link {
            a: parse_endpoint(&self.a)?,
            b: parse_endpoint(&self.b)?,
        })
    }
}

/// TOML shape for one post-bootstrap check.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckManifest {
    /// Check type discriminator.
    #[serde(rename = "type")]
    check_type: String,
    /// Router name to check.
    router: String,
    /// Whether to skip endpoints unavailable on this `RouterOS` version.
    #[serde(default)]
    allow_unsupported: bool,
    /// Raw command for `command-rows` checks.
    command: Option<String>,
    /// Minimum accepted row count for `command-rows` checks.
    min_rows: Option<usize>,
}

impl CheckManifest {
    /// Convert the TOML shape into a check.
    fn try_into_check(self) -> Result<Check> {
        match self.check_type.as_str() {
            "all-print-commands" => Ok(Check::AllPrintCommands {
                router: self.router,
                allow_unsupported: self.allow_unsupported,
            }),
            "command-rows" => Ok(Check::CommandRows {
                router: self.router,
                command: self
                    .command
                    .ok_or_else(|| Error::Manifest("missing required `command`".to_owned()))?,
                min_rows: self.min_rows.unwrap_or(1),
            }),
            check_type => Err(Error::Manifest(format!("unsupported check type `{check_type}`"))),
        }
    }
}

/// Default router memory in MiB.
const fn default_memory_mib() -> u16 {
    256
}

/// Default router CPU count.
const fn default_cpus() -> u8 {
    1
}

/// Parse a `router:interface` endpoint string.
fn parse_endpoint(value: &str) -> Result<Endpoint> {
    let (router, interface) = value
        .split_once(':')
        .ok_or_else(|| Error::Manifest(format!("endpoint `{value}` must use router:interface")))?;
    Ok(Endpoint {
        router: router.to_owned(),
        interface: interface.to_owned(),
    })
}

/// Parse a bootstrap command string into command path and attributes.
fn parse_router_command(value: &str) -> Result<RouterCommand> {
    let mut parts = value.split_whitespace();
    let command = parts
        .next()
        .ok_or_else(|| Error::Manifest("bootstrap command must not be empty".to_owned()))?;
    if !command.starts_with('/') {
        return Err(Error::Manifest(format!(
            "bootstrap command `{command}` must start with `/`"
        )));
    }

    let mut attributes = Vec::new();
    for part in parts {
        let (key, value) = part
            .split_once('=')
            .map_or_else(|| (part, None), |(key, value)| (key, Some(value)));
        if key.is_empty() {
            return Err(Error::Manifest(format!(
                "bootstrap command `{command}` has an empty attribute key"
            )));
        }
        attributes.push(CommandAttribute {
            key: key.to_owned(),
            value: value.map(ToOwned::to_owned),
        });
    }

    Ok(RouterCommand {
        command: command.to_owned(),
        attributes,
    })
}

/// Validate cross-references and deterministic naming assumptions.
fn validate_topology(topology: &Topology) -> Result<()> {
    if topology.routers.is_empty() {
        return Err(Error::Manifest("topology must declare at least one router".to_owned()));
    }
    for router in &topology.routers {
        if router.name.is_empty() {
            return Err(Error::Manifest("router name must not be empty".to_owned()));
        }
        if router.version.is_empty() {
            return Err(Error::Manifest(format!(
                "router {} version must not be empty",
                router.name
            )));
        }
    }
    for link in &topology.links {
        topology.router(&link.a.router)?;
        topology.router(&link.b.router)?;
        link_interface_index(&link.a.router, link)?;
        link_interface_index(&link.b.router, link)?;
    }
    for check in &topology.checks {
        match check {
            Check::AllPrintCommands { router, .. } | Check::CommandRows { router, .. } => {
                topology.router(router)?;
            }
        }
    }
    Ok(())
}
