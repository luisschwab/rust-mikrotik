#![deny(missing_docs)]

//! Local QEMU/CHR simulation harness for `rust-mikrotik`.
//!
//! This crate is intentionally internal. It parses deterministic topology
//! manifests, prepares cached CHR images under `.chr-cache/images`, starts
//! per-run qcow2 overlays under `.chr-cache/runs`, applies bootstrap commands, and
//! runs client-side assertions against the routers.

use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

/// RouterOS CHR version catalog and image URL helpers.
mod catalog;
/// Error and result types for simnet operations.
mod error;
/// Mermaid topology diagram rendering.
mod mermaid;
/// Deterministic topology manifest parser.
mod parser;
/// QEMU host probing and argument construction.
mod qemu;
/// Topology execution lifecycle.
mod runner;
/// Simulation report helpers.
mod simulation_report;
/// Public topology manifest types.
mod topology;
/// RouterOS catalog listing for CHR simulation.
mod versions;

#[cfg(test)]
mod tests;

pub use catalog::ChrArch;
pub use catalog::ROUTEROS_VERSIONS;
pub use catalog::RouterOsChannel;
pub use catalog::RouterOsVersion;
pub use error::Error;
pub use error::Result;
pub use runner::SpawnedNode;
pub use runner::SpawnedTopology;
pub use topology::Check;
pub use topology::CommandAttribute;
pub use topology::Endpoint;
pub use topology::Link;
pub use topology::Router;
pub use topology::RouterCommand;
pub use topology::RunOptions;
pub use topology::Topology;
pub use versions::VersionImage;
pub use versions::VersionList;
pub use versions::version_list;

/// Environment variable that enables simnet integration tests.
pub const ENABLE_ENV: &str = "MIKROTIK_SIMNET";

/// Root directory for cached CHR images and local simnet runtime state.
const CACHE_DIR: &str = ".chr-cache";
/// Directory for cached CHR base images.
const IMAGES_DIR: &str = ".chr-cache/images";
/// Directory for per-invocation overlays, sockets, logs, and pid files.
const RUNS_DIR: &str = ".chr-cache/runs";
/// Workspace-level directory containing bundled topology manifests.
const TOPOLOGIES_DIR: &str = "../topologies";
/// Maximum time to wait for API login after starting routers.
const DEFAULT_BOOT_TIMEOUT: Duration = Duration::from_secs(600);
/// Default CHR admin username.
const DEFAULT_USERNAME: &str = "admin";
/// Default CHR admin password before bootstrap.
const DEFAULT_PASSWORD: &str = "";

/// Run a topology manifest from disk.
///
/// # Errors
///
/// Returns an error if the manifest is invalid, required host tools are
/// unavailable, image preparation fails, routers cannot boot, or checks fail.
pub async fn run_topology(path: impl AsRef<Path>) -> Result<()> {
    run_topology_with_options(path, RunOptions::default()).await
}

/// Run a topology manifest from disk with explicit runtime options.
///
/// # Errors
///
/// Returns an error if the manifest is invalid, required host tools are
/// unavailable, image preparation fails, routers cannot boot, or checks fail.
pub async fn run_topology_with_options(path: impl AsRef<Path>, options: RunOptions) -> Result<()> {
    let path = path.as_ref();
    let topology_path = resolve_topology_path(path);
    let topology = Topology::read(&topology_path)?;

    runner::SimulatedNetwork::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")), topology)
        .run(options)
        .await
}

/// Spawn a topology manifest from disk and keep it running until the returned handle is dropped.
///
/// # Errors
///
/// Returns an error if the manifest is invalid, required host tools are
/// unavailable, image preparation fails, routers cannot boot, or bootstrap fails.
pub async fn spawn_topology(path: impl AsRef<Path>) -> Result<SpawnedTopology> {
    spawn_topology_with_options(path, RunOptions::default().without_checks()).await
}

/// Spawn a topology manifest from disk with explicit runtime options.
///
/// # Errors
///
/// Returns an error if the manifest is invalid, required host tools are
/// unavailable, image preparation fails, routers cannot boot, bootstrap fails,
/// or enabled checks fail.
pub async fn spawn_topology_with_options(path: impl AsRef<Path>, options: RunOptions) -> Result<SpawnedTopology> {
    let path = path.as_ref();
    let topology_path = resolve_topology_path(path);
    let topology = Topology::read(&topology_path)?;

    runner::SimulatedNetwork::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")), topology)
        .spawn(options)
        .await
}

/// Render a topology manifest from disk as a Mermaid diagram.
///
/// # Errors
///
/// Returns an error if the manifest path cannot be read or parsed.
pub fn topology_mermaid(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let topology_path = resolve_topology_path(path);
    let topology = Topology::read(&topology_path)?;
    Ok(mermaid::render_topology_mermaid(&topology))
}

/// Resolve a topology path, accepting bundled topology file names.
fn resolve_topology_path(path: &Path) -> PathBuf {
    if path.exists() {
        return path.to_owned();
    }

    path.file_name().map_or_else(
        || path.to_owned(),
        |file_name| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(TOPOLOGIES_DIR)
                .join(file_name)
        },
    )
}

/// Return whether explicit simnet tests are enabled.
#[must_use]
pub fn enabled_from_env() -> bool {
    env::var(ENABLE_ENV).is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}
