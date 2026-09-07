//! `RouterOS` response-field schema auditing.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::PoisonError;

use mikrotik_client::builder::Protocol;
use mikrotik_client::client::AuditedRows;
use mikrotik_client::client::Client;
use mikrotik_client::commands::PrintCommand;
use mikrotik_client::error::DecodeError;
use mikrotik_qemu_runner::MikrotikDConf;
use mikrotik_qemu_runner::Scenario;
use mikrotik_qemu_runner::ScenarioConf;
use mikrotik_types::target::DeviceTarget;
use serde::Deserialize;
use serde::Serialize;
use tokio::task::JoinSet;

use crate::config::DEFAULT_COMMAND_TIMEOUT;
use crate::config::DEFAULT_CONNECT_TIMEOUT;
use crate::connector::builder_from_target;
use crate::error::Error;
use crate::error::Result;
use crate::snapshot::collector::EndpointCollector;
use crate::snapshot::sections::collect_router_os_snapshot;

/// Current machine-readable field-baseline format.
const SCHEMA_BASELINE_FORMAT: u32 = 1;
/// Maximum number of devices queried concurrently during a scenario audit.
const SCHEMA_AUDIT_CONCURRENCY: usize = 4;

/// Field evidence for one `RouterOS` version, command, and Rust model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaObservation {
    /// `RouterOS` version that returned these fields.
    pub router_os_version: String,
    /// Exact `RouterOS` print command.
    pub command: String,
    /// Fully qualified Rust response-model name.
    pub model: String,
    /// Union of fields present in successful raw rows.
    pub observed_fields: BTreeSet<String>,
    /// Fields present in raw rows but not consumed by the model.
    pub ignored_fields: BTreeSet<String>,
}

/// Deterministically ordered observations from one or more schema-audit targets.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaAuditReport {
    /// Observations sorted by version, command, and model.
    pub observations: Vec<SchemaObservation>,
    /// Typed decoding failures sorted by version, command, model, and row.
    pub decode_failures: Vec<SchemaDecodeFailure>,
}

impl SchemaAuditReport {
    /// Merge another report, unioning fields for matching version/command/model entries.
    pub fn merge(&mut self, other: Self) {
        let mut merged = self
            .observations
            .drain(..)
            .map(|observation| (observation.key(), observation))
            .collect::<BTreeMap<_, _>>();
        for observation in other.observations {
            let key = observation.key();
            let entry = merged.entry(key).or_insert_with(|| SchemaObservation {
                router_os_version: observation.router_os_version.clone(),
                command: observation.command.clone(),
                model: observation.model.clone(),
                observed_fields: BTreeSet::new(),
                ignored_fields: BTreeSet::new(),
            });
            entry.observed_fields.extend(observation.observed_fields);
            entry.ignored_fields.extend(observation.ignored_fields);
        }
        self.observations = merged.into_values().collect();
        self.decode_failures.extend(other.decode_failures);
        self.decode_failures.sort();
        self.decode_failures.dedup();
    }

    /// Return whether every field was consumed and every observed row decoded.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.observations
            .iter()
            .all(|observation| observation.ignored_fields.is_empty())
            && self.decode_failures.is_empty()
    }

    /// Format ignored fields grouped by `RouterOS` version, command, and model.
    #[must_use]
    pub fn format_ignored_fields(&self) -> String {
        let mut output = String::new();
        for observation in &self.observations {
            if observation.ignored_fields.is_empty() {
                continue;
            }
            output.push_str(&format!(
                "RouterOS {} | {} | {}\n",
                observation.router_os_version, observation.command, observation.model
            ));
            for field in &observation.ignored_fields {
                output.push_str(&format!("  - {field}\n"));
            }
        }
        output
    }

    /// Format all ignored fields and typed decoding failures deterministically.
    #[must_use]
    pub fn format_problems(&self) -> String {
        let mut output = self.format_ignored_fields();
        for failure in &self.decode_failures {
            output.push_str(&format!(
                "RouterOS {} | {} | {} | row {}\n  - decode error: {}\n",
                failure.router_os_version, failure.command, failure.model, failure.row_index, failure.message
            ));
        }
        output
    }

    /// Convert observed-field unions into the stable baseline representation.
    #[must_use]
    pub fn field_baseline(&self) -> SchemaFieldBaseline {
        SchemaFieldBaseline {
            format: SCHEMA_BASELINE_FORMAT,
            observations: self
                .observations
                .iter()
                .map(|observation| SchemaFieldObservation {
                    router_os_version: observation.router_os_version.clone(),
                    command: observation.command.clone(),
                    model: observation.model.clone(),
                    fields: observation.observed_fields.clone(),
                })
                .collect(),
        }
    }
}

/// A raw `RouterOS` row whose known fields did not match their modeled value types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SchemaDecodeFailure {
    /// `RouterOS` version that returned the malformed-for-model row.
    pub router_os_version: String,
    /// Exact `RouterOS` print command.
    pub command: String,
    /// Fully qualified Rust response-model name.
    pub model: String,
    /// Zero-based row index within the command response.
    pub row_index: usize,
    /// Deserializer error without the potentially sensitive raw row.
    pub message: String,
}

impl SchemaObservation {
    /// Return the deterministic grouping key for this observation.
    fn key(&self) -> (String, String, String) {
        (self.router_os_version.clone(), self.command.clone(), self.model.clone())
    }
}

/// Machine-readable baseline of fields observed from supported `RouterOS` versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaFieldBaseline {
    /// Baseline serialization format version.
    pub format: u32,
    /// Observed field unions sorted by version, command, and model.
    pub observations: Vec<SchemaFieldObservation>,
}

/// Baseline field union for one `RouterOS` version, command, and Rust model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaFieldObservation {
    /// `RouterOS` version that returned these fields.
    pub router_os_version: String,
    /// Exact `RouterOS` print command.
    pub command: String,
    /// Fully qualified Rust response-model name.
    pub model: String,
    /// Union of fields present in successful raw rows.
    pub fields: BTreeSet<String>,
}

/// Collect a full typed snapshot through the permissive schema-audit decoder.
///
/// Unsupported optional commands remain endpoint-local snapshot failures and do
/// not prevent the remaining endpoints from being audited.
///
/// # Errors
///
/// Returns an error when the target cannot be connected, a required endpoint
/// fails, or an otherwise malformed row cannot be decoded.
pub async fn audit_target_schema(target: &DeviceTarget, router_os_version: &str) -> Result<SchemaAuditReport> {
    let builder = builder_from_target(target, Protocol::Api, DEFAULT_CONNECT_TIMEOUT);
    let client = Client::connect(builder).await?;
    let recorder = SchemaAuditRecorder::new(router_os_version);
    let target_address = target.address.to_string();
    let collector =
        EndpointCollector::new_with_schema_audit(&target_address, &client, DEFAULT_COMMAND_TIMEOUT, &recorder);
    let _snapshot = Box::pin(collect_router_os_snapshot(&collector)).await?;
    Ok(recorder.report())
}

/// Audit every router declared by a CHR scenario manifest.
///
/// Routers are booted independently in a bounded pool. Topology links are not
/// needed for response-field coverage, and isolation prevents one failed link
/// peer from terminating other guests. Bootstrap references to topology-only
/// Ethernet interfaces are redirected to the management interface.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read, the scenario cannot be
/// started, a worker cannot be joined, or one target audit fails.
pub async fn audit_scenario_schema(path: impl AsRef<Path>) -> Result<SchemaAuditReport> {
    let scenario_conf = ScenarioConf::read(path).map_err(qemu_error)?;
    let mut pending = scenario_conf
        .devices
        .into_iter()
        .map(|config| IsolatedAuditInput {
            config: redirect_topology_interfaces(config),
            allow_software_emulation: scenario_conf.allow_software_emulation,
        })
        .collect::<VecDeque<_>>();
    let mut workers = JoinSet::new();
    let mut report = SchemaAuditReport::default();

    for _ in 0..SCHEMA_AUDIT_CONCURRENCY {
        if let Some(input) = pending.pop_front() {
            spawn_audit_worker(&mut workers, input);
        }
    }

    while let Some(result) = workers.join_next().await {
        report.merge(result.map_err(|error| Error::Io(io::Error::other(error)))??);
        if let Some(input) = pending.pop_front() {
            spawn_audit_worker(&mut workers, input);
        }
    }

    Ok(report)
}

/// One router prepared for a link-free audit run.
struct IsolatedAuditInput {
    /// Router and bootstrap configuration.
    config: MikrotikDConf,
    /// Whether the manifest allows software emulation.
    allow_software_emulation: bool,
}

/// Spawn one owned, isolated router audit task.
fn spawn_audit_worker(workers: &mut JoinSet<Result<SchemaAuditReport>>, input: IsolatedAuditInput) {
    workers.spawn(async move {
        let version = input.config.version.as_str();
        let scenario_conf = ScenarioConf::new(format!("schema-audit-{}", input.config.name))
            .with_software_emulation(input.allow_software_emulation)
            .with_device(&input.config);
        let scenario = Scenario::new_with_conf(&scenario_conf).await.map_err(qemu_error)?;
        let target = scenario
            .targets()
            .into_iter()
            .next()
            .ok_or_else(|| Error::Io(io::Error::other("isolated schema-audit scenario has no target")))?;
        audit_target_schema(&target, version).await
    });
}

/// Redirect link-only `etherN` references to the isolated guest's `ether1`.
fn redirect_topology_interfaces(mut config: MikrotikDConf) -> MikrotikDConf {
    for command in &mut config.bootstrap {
        for attribute in &mut command.attributes {
            if let Some(value) = &mut attribute.value {
                for interface in ["ether2", "ether3", "ether4"] {
                    *value = value.replace(interface, "ether1");
                }
            }
        }
    }
    config
}

/// Convert QEMU-runner failures into the crawler's process-I/O context.
fn qemu_error(error: mikrotik_qemu_runner::Error) -> Error {
    Error::Io(io::Error::other(error))
}

/// Interior-mutable field recorder shared by every endpoint in one target audit.
pub(crate) struct SchemaAuditRecorder {
    /// `RouterOS` version attached to every observation.
    router_os_version: String,
    /// Command/model keyed observed and ignored field unions.
    entries: Mutex<BTreeMap<(String, String), RecordedFields>>,
    /// Typed row failures found while auditing.
    decode_failures: Mutex<BTreeSet<SchemaDecodeFailure>>,
}

impl SchemaAuditRecorder {
    /// Build an empty recorder for one `RouterOS` version.
    fn new(router_os_version: &str) -> Self {
        Self {
            router_os_version: router_os_version.to_owned(),
            entries: Mutex::new(BTreeMap::new()),
            decode_failures: Mutex::new(BTreeSet::new()),
        }
    }

    /// Record a known field whose value could not be decoded by its model.
    pub(crate) fn record_decode_failure(&self, error: &DecodeError) {
        let mut failures = self.decode_failures.lock().unwrap_or_else(PoisonError::into_inner);
        failures.insert(SchemaDecodeFailure {
            router_os_version: self.router_os_version.clone(),
            command: error.command().to_owned(),
            model: error.model().to_owned(),
            row_index: error.row_index(),
            message: error.message().to_owned(),
        });
    }

    /// Record all raw-row field evidence from one typed print command.
    pub(crate) fn record<T>(&self, command: PrintCommand, audited: &AuditedRows<T>) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = entries
            .entry((command.to_string(), audited.model().to_owned()))
            .or_default();
        for row in audited.row_fields() {
            entry.observed_fields.extend(row.observed_fields().iter().cloned());
            entry.ignored_fields.extend(row.ignored_fields().iter().cloned());
        }
    }

    /// Snapshot the current observations in deterministic order.
    fn report(&self) -> SchemaAuditReport {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let decode_failures = self.decode_failures.lock().unwrap_or_else(PoisonError::into_inner);
        SchemaAuditReport {
            observations: entries
                .iter()
                .map(|((command, model), fields)| SchemaObservation {
                    router_os_version: self.router_os_version.clone(),
                    command: command.clone(),
                    model: model.clone(),
                    observed_fields: fields.observed_fields.clone(),
                    ignored_fields: fields.ignored_fields.clone(),
                })
                .collect(),
            decode_failures: decode_failures.iter().cloned().collect(),
        }
    }
}

/// Accumulated field unions for one command/model pair.
#[derive(Default)]
struct RecordedFields {
    /// Union of raw fields returned across all rows.
    observed_fields: BTreeSet<String>,
    /// Union of raw fields not consumed by the model.
    ignored_fields: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(version: &str, field: &str, ignored: bool) -> SchemaObservation {
        SchemaObservation {
            router_os_version: version.to_owned(),
            command: "/system/identity/print".to_owned(),
            model: "mikrotik_types::api::system::Identity".to_owned(),
            observed_fields: BTreeSet::from([field.to_owned()]),
            ignored_fields: if ignored {
                BTreeSet::from([field.to_owned()])
            } else {
                BTreeSet::new()
            },
        }
    }

    #[test]
    fn reports_merge_and_format_ignored_fields_deterministically() {
        let mut report = SchemaAuditReport {
            observations: vec![observation("7.24.2", "name", false)],
            decode_failures: Vec::new(),
        };
        report.merge(SchemaAuditReport {
            observations: vec![observation("7.24.2", "future-field", true)],
            decode_failures: Vec::new(),
        });

        assert!(!report.is_clean());
        assert_eq!(report.observations.len(), 1);
        assert_eq!(
            report.observations[0].observed_fields,
            BTreeSet::from(["future-field".to_owned(), "name".to_owned()])
        );
        assert_eq!(
            report.format_ignored_fields(),
            "RouterOS 7.24.2 | /system/identity/print | mikrotik_types::api::system::Identity\n  - future-field\n"
        );
        assert_eq!(report.field_baseline().format, SCHEMA_BASELINE_FORMAT);
    }

    #[test]
    fn decode_failures_make_a_report_unclean() {
        let report = SchemaAuditReport {
            observations: Vec::new(),
            decode_failures: vec![SchemaDecodeFailure {
                router_os_version: "7.24.2".to_owned(),
                command: "/system/license/print".to_owned(),
                model: "mikrotik_types::api::system::License".to_owned(),
                row_index: 0,
                message: "invalid digit found in string".to_owned(),
            }],
        };

        assert!(!report.is_clean());
        assert!(
            report
                .format_problems()
                .contains("decode error: invalid digit found in string")
        );
    }
}
