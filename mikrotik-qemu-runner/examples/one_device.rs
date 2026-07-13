//! Spawn one CHR device, query `/system/resource/print`, and shut it down.

use std::path::PathBuf;

use mikrotik_common::info_with_label;
use mikrotik_qemu_runner::MikrotikD;
use mikrotik_qemu_runner::Result;
use mikrotik_qemu_runner::ScenarioConf;

#[tokio::main]
async fn main() -> Result<()> {
    mikrotik_common::logging::init_tracing();

    let scenario_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join("one-router.toml");
    let scenario_conf = ScenarioConf::read(scenario_path)?;
    let mikrotikd_conf = scenario_conf
        .devices
        .first()
        .expect("one-router scenario has one device");

    info_with_label!(mikrotikd_conf.name, "Starting device");
    let device = MikrotikD::new_with_conf(mikrotikd_conf).await?;
    info_with_label!(device.name(), "API is ready at {}", device.api_socket());

    let mikrotikd_client = device.client().await?;

    info_with_label!(device.name(), "Querying /system/resource/print");
    let response = mikrotikd_client.call("/system/resource/print", &[]).await?;

    info_with_label!(device.name(), "Resource query completed with {} row(s)", response.len());
    info_with_label!(device.name(), "Run artifacts written to {}", device.run_dir().display());

    Ok(())
}
