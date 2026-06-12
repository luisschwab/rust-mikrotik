//! Public topology manifest types.

use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::error::Result;
use crate::parser::parse_topology;

/// Parsed topology manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topology {
    /// Human-readable topology name.
    pub name: String,
    /// Allow software QEMU emulation when hardware acceleration is unavailable.
    pub allow_software_emulation: bool,
    /// Routers in deterministic startup order.
    pub routers: Vec<Router>,
    /// Links between router interfaces.
    pub links: Vec<Link>,
    /// Expectations to run after bootstrap.
    pub checks: Vec<Check>,
}

/// Runtime options for executing a topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunOptions {
    /// Exit immediately after all topology checks pass.
    pub exit_after_checks: bool,
}

impl RunOptions {
    /// Return the default interactive behavior: keep the topology running until Ctrl-C.
    #[must_use]
    pub const fn interactive() -> Self {
        Self {
            exit_after_checks: false,
        }
    }

    /// Return CI behavior: stop router processes after checks pass.
    #[must_use]
    pub const fn exit_after_checks() -> Self {
        Self {
            exit_after_checks: true,
        }
    }
}

/// Router declared by a topology manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Router {
    /// Stable router name.
    pub name: String,
    /// `RouterOS` version used for the CHR image.
    pub version: String,
    /// Memory in MiB.
    pub memory_mib: u16,
    /// CPU count.
    pub cpus: u8,
    /// Commands applied after first API login.
    pub bootstrap: Vec<RouterCommand>,
}

/// Raw `RouterOS` command with optional attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterCommand {
    /// `RouterOS` API command path.
    pub command: String,
    /// Command attributes in manifest order.
    pub attributes: Vec<CommandAttribute>,
}

/// One `RouterOS` command attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAttribute {
    /// Attribute key without leading `=`.
    pub key: String,
    /// Attribute value, or `None` for a flag attribute.
    pub value: Option<String>,
}

/// Point-to-point link between two router interfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// First endpoint as `router:interface`.
    pub a: Endpoint,
    /// Second endpoint as `router:interface`.
    pub b: Endpoint,
}

/// One link endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Router name.
    pub router: String,
    /// Interface name.
    pub interface: String,
}

/// Post-bootstrap check declared by a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// Run every typed print endpoint method currently shared with the live-router test.
    AllPrintMethods {
        /// Router name to check.
        router: String,
        /// Whether to skip endpoints unavailable on this `RouterOS` version.
        allow_unsupported: bool,
    },
    /// Execute a raw command and require at least `min_rows` reply rows.
    CommandRows {
        /// Router name to check.
        router: String,
        /// `RouterOS` API command path.
        command: String,
        /// Minimum accepted row count.
        min_rows: usize,
    },
}

impl Topology {
    /// Read and parse a topology manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or when manifest contents
    /// do not match the supported deterministic TOML subset.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Self::parse(&contents)
    }

    /// Parse a topology manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when manifest contents do not match the supported
    /// deterministic TOML subset.
    pub fn parse(contents: &str) -> Result<Self> {
        parse_topology(contents)
    }

    /// Look up a router by name.
    pub(crate) fn router(&self, name: &str) -> Result<&Router> {
        self.routers
            .iter()
            .find(|router| router.name == name)
            .ok_or_else(|| Error::Manifest(format!("unknown router `{name}`")))
    }
}
