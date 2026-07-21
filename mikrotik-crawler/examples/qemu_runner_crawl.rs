//! Crawl a small QEMU runner scenario through the normal crawler API.

use std::io;
use std::path::PathBuf;

use mikrotik_common::info_with_label;
use mikrotik_common::logging::init_tracing;
use mikrotik_crawler::Crawler;
use mikrotik_crawler::error::Error;
use mikrotik_crawler::error::Result;
use mikrotik_qemu_runner::Scenario;
use mikrotik_qemu_runner::ScenarioConf;
use tracing::Level;

/// Worker stack size for large `RouterOS` snapshot futures.
const TOKIO_WORKER_STACK_SIZE: usize = 16 * 1024 * 1024;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(TOKIO_WORKER_STACK_SIZE)
        .build()
        .map_err(Error::Io)?
        .block_on(run())
}

/// Run the example crawl.
async fn run() -> Result<()> {
    init_tracing(Level::INFO);

    let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../mikrotik-qemu-runner/scenarios")
        .join("two-router.toml");
    let scenario_conf = ScenarioConf::read(scenario_path).map_err(|error| Error::Io(io::Error::other(error)))?;
    info_with_label!(
        scenario_conf.name,
        "Starting scenario with {} device(s) and {} link(s)",
        scenario_conf.devices.len(),
        scenario_conf.links.len()
    );
    let scenario = Scenario::new_with_conf(&scenario_conf)
        .await
        .map_err(|error| Error::Io(io::Error::other(error)))?;

    info_with_label!("Crawler", "Running with {} target(s)", scenario.targets().len());
    let report = Crawler::default().crawl_many(scenario.targets()).await?;

    info_with_label!(
        "Crawler",
        "Completed with {} node(s), {} edge(s), and {} failed target(s)",
        report.graph.nodes.len(),
        report.graph.edges.len(),
        report.failed_targets.len()
    );
    info_with_label!(
        scenario_conf.name,
        "Run artifacts written to {}",
        scenario.run_dir().display()
    );

    Ok(())
}
