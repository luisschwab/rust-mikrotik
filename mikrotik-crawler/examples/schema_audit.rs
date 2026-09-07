//! Audit supported CHR responses against the checked-in field baseline.

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use argh::FromArgs;
use mikrotik_common::logging::init_tracing;
use mikrotik_crawler::schema_audit::SchemaAuditReport;
use mikrotik_crawler::schema_audit::SchemaFieldBaseline;
use mikrotik_crawler::schema_audit::audit_scenario_schema;
use tracing::Level;

/// Audit supported CHR response fields.
#[derive(FromArgs)]
struct Args {
    /// replace the checked-in observed-field baseline
    #[argh(switch)]
    update: bool,
    /// audit only these scenario manifests instead of the complete baseline set
    #[argh(option)]
    manifest: Vec<PathBuf>,
    /// write the complete audit report as JSON before validation
    #[argh(option)]
    output: Option<PathBuf>,
}

/// Run both the representative configuration and complete version audits.
async fn collect_report(workspace: &Path, manifests: &[PathBuf]) -> Result<SchemaAuditReport, Box<dyn Error>> {
    let default_manifests = [
        workspace.join("mikrotik-qemu-runner/scenarios/config-stress.toml"),
        workspace.join("mikrotik-qemu-runner/scenarios/version-stress-test.toml"),
    ];
    let manifests = if manifests.is_empty() {
        default_manifests.as_slice()
    } else {
        manifests
    };
    let mut report = SchemaAuditReport::default();
    for manifest in manifests {
        report.merge(audit_scenario_schema(manifest).await?);
    }
    Ok(report)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing(Level::INFO);
    let args: Args = argh::from_env();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| io::Error::other("crawler crate has no workspace parent"))?
        .to_path_buf();
    let baseline_path = workspace.join("mikrotik-crawler/tests/fixtures/routeros-fields.json");
    let report = collect_report(&workspace, &args.manifest).await?;

    if let Some(output) = &args.output {
        let mut json = serde_json::to_string_pretty(&report)?;
        json.push('\n');
        fs::write(output, json)?;
        println!("wrote {}", output.display());
    }

    if !report.is_clean() {
        return Err(io::Error::other(format!(
            "RouterOS response schema problems:\n{}",
            report.format_problems()
        ))
        .into());
    }

    let baseline = report.field_baseline();
    if !args.manifest.is_empty() {
        println!("{}", serde_json::to_string_pretty(&baseline)?);
        return Ok(());
    }
    if args.update {
        let mut json = serde_json::to_string_pretty(&baseline)?;
        json.push('\n');
        fs::write(&baseline_path, json)?;
        println!("updated {}", baseline_path.display());
        return Ok(());
    }

    let expected = serde_json::from_str::<SchemaFieldBaseline>(&fs::read_to_string(&baseline_path)?)?;
    if baseline != expected {
        return Err(io::Error::other(format!(
            "RouterOS response-field baseline changed; inspect the schema diff and run with --update when intentional\n{}",
            serde_json::to_string_pretty(&baseline)?
        ))
        .into());
    }

    println!(
        "RouterOS schema audit passed with {} version/command/model observations",
        baseline.observations.len()
    );
    Ok(())
}
