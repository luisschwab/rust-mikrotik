//! Spawn two linked CHR devices and confirm both API clients are reachable.

use std::path::PathBuf;

use mikrotik_common::info_with_label;
use mikrotik_qemu_runner::Result;
use mikrotik_qemu_runner::Scenario;
use mikrotik_qemu_runner::ScenarioConf;

#[tokio::main]
async fn main() -> Result<()> {
    mikrotik_common::logging::init_tracing();

    let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join("two-router.toml");
    let scenario_conf = ScenarioConf::read(scenario_path)?;

    info_with_label!(
        scenario_conf.name,
        "Starting scenario with {} device(s) and {} link(s)",
        scenario_conf.devices.len(),
        scenario_conf.links.len()
    );
    let scenario = Scenario::new_with_conf(&scenario_conf).await?;
    info_with_label!(scenario.name(), "Scenario API is ready");

    for device in scenario.devices() {
        let client = device.client().await?;
        let rows = client.call("/system/resource/print", &[]).await?;
        info_with_label!(device.name(), "API reachable with {} resource row(s)", rows.len());
    }
    info_with_label!(
        scenario.name(),
        "Run artifacts written to {}",
        scenario.run_dir().display()
    );

    Ok(())
}
