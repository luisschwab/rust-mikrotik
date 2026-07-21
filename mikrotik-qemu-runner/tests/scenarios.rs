//! Live scenario tests for bundled QEMU runner scenario files.

use std::path::Path;
use std::sync::Once;

use mikrotik_common::info_with_label;
use mikrotik_common::logging::init_tracing;
use mikrotik_qemu_runner::Result;
use mikrotik_qemu_runner::Scenario;
use mikrotik_qemu_runner::ScenarioConf;
use tracing::Level;

/// Initialize test logging once per test binary.
static INIT_TRACING: Once = Once::new();

/// Bundled scenario manifest file names.
const SCENARIOS: &[&str] = &[
    "one-router.toml",
    "two-router.toml",
    "three-router-with-bgp.toml",
    "isp-network.toml",
    "config-stress.toml",
    "version-stress-test.toml",
];

/// Spawn one bundled scenario and let `Drop` clean it up.
async fn spawn_scenario(file_name: &str) -> Result<()> {
    INIT_TRACING.call_once(|| init_tracing(Level::INFO));

    let scenario_conf = ScenarioConf::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(file_name))?;
    info_with_label!(
        scenario_conf.name,
        "Starting scenario with {} device(s) and {} link(s)",
        scenario_conf.devices.len(),
        scenario_conf.links.len()
    );

    let scenario = Scenario::new_with_conf(&scenario_conf).await?;
    assert_eq!(scenario.name(), scenario_conf.name);
    assert_eq!(scenario.devices().len(), scenario_conf.devices.len());
    Ok(())
}

#[test]
fn bundled_scenarios_parse() {
    for file_name in SCENARIOS {
        let scenario_conf = ScenarioConf::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios").join(file_name))
            .expect("scenario should parse");
        assert!(!scenario_conf.name.is_empty());
        assert!(!scenario_conf.devices.is_empty());
    }
}

#[tokio::test]
#[ignore = "Boots CHR in QEMU"]
async fn one_router_scenario() -> Result<()> {
    spawn_scenario("one-router.toml").await
}

#[tokio::test]
#[ignore = "Boots multiple CHRs in QEMU"]
async fn two_router_scenario() -> Result<()> {
    spawn_scenario("two-router.toml").await
}

#[tokio::test]
#[ignore = "Boots multiple CHRs in QEMU"]
async fn three_router_bgp_scenario() -> Result<()> {
    spawn_scenario("three-router-with-bgp.toml").await
}

#[tokio::test]
#[ignore = "Boots multiple CHRs in QEMU"]
async fn isp_network_scenario() -> Result<()> {
    spawn_scenario("isp-network.toml").await
}

#[tokio::test]
#[ignore = "Boots many CHRs in QEMU"]
async fn config_stress_scenario() -> Result<()> {
    spawn_scenario("config-stress.toml").await
}

#[tokio::test]
#[ignore = "Boots every cataloged RouterOS version in QEMU"]
async fn version_stress_test_scenario() -> Result<()> {
    spawn_scenario("version-stress-test.toml").await
}
