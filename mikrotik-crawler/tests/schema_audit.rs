//! Live CHR response-field schema audit.

use std::path::PathBuf;
use std::sync::Once;

use mikrotik_common::logging::init_tracing;
use mikrotik_crawler::schema_audit::SchemaFieldBaseline;
use mikrotik_crawler::schema_audit::audit_scenario_schema;
use tracing::Level;

/// Initialize test logging once per test binary.
static INIT_TRACING: Once = Once::new();

/// Audit every supported version plus representative configured endpoint rows.
#[tokio::test]
#[ignore = "Boots the config-stress and every-version CHR scenarios in QEMU"]
async fn supported_routeros_response_fields_match_baseline() {
    INIT_TRACING.call_once(|| init_tracing(Level::INFO));
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let scenarios = workspace.join("mikrotik-qemu-runner/scenarios");
    let mut report = audit_scenario_schema(scenarios.join("config-stress.toml"))
        .await
        .expect("config-stress schema audit should complete");
    report.merge(
        audit_scenario_schema(scenarios.join("version-stress-test.toml"))
            .await
            .expect("version-stress schema audit should complete"),
    );

    assert!(
        report.is_clean(),
        "RouterOS response schema problems:\n{}",
        report.format_problems()
    );

    let expected = serde_json::from_str::<SchemaFieldBaseline>(include_str!("fixtures/routeros-fields.json"))
        .expect("field baseline should parse");
    assert_eq!(report.field_baseline(), expected);
}

#[test]
fn checked_in_routeros_field_baseline_is_well_formed() {
    let baseline = serde_json::from_str::<SchemaFieldBaseline>(include_str!("fixtures/routeros-fields.json"))
        .expect("field baseline should parse");
    assert_eq!(baseline.format, 1);
}
