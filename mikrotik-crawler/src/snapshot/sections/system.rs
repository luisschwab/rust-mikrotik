//! `/system` snapshot collection.

use mikrotik_client::commands::PrintCommand;
use mikrotik_client::commands::system::System;
use mikrotik_types::device::SystemSnapshot;

use super::EndpointCollector;
use crate::error::Result;

/// Collect required identity data and optional `/system` endpoints.
pub(super) async fn collect(collector: &EndpointCollector<'_>) -> Result<SystemSnapshot> {
    Ok(SystemSnapshot {
        identity: collector.required_first(command(System::Identity)).await?,
        resource: collector.required_first(command(System::Resource)).await?,
        routerboard: collector.required_first(command(System::Routerboard)).await?,
        clock: collector.optional_first(command(System::Clock)).await,
        packages: collector.optional_many(command(System::Package)).await,
        package_updates: collector.optional_many(command(System::PackageUpdate)).await,
        health: collector.optional_many(command(System::Health)).await,
        resource_cpus: collector.optional_many(command(System::ResourceCpu)).await,
        resource_hardware: collector.optional_many(command(System::ResourceHardware)).await,
        resource_irqs: collector.optional_many(command(System::ResourceIrq)).await,
        resource_usb_settings: collector.optional_many(command(System::ResourceUsbSettings)).await,
        routerboard_settings: collector.optional_many(command(System::RouterboardSettings)).await,
        routerboard_reset_buttons: collector.optional_many(command(System::RouterboardResetButton)).await,
        device_modes: collector.optional_many(command(System::DeviceMode)).await,
        history_entries: collector.optional_many(command(System::HistoryEntry)).await,
        leds: collector.optional_many(command(System::Led)).await,
        licenses: collector.optional_many(command(System::License)).await,
        log_entries: collector.optional_many(command(System::LogEntry)).await,
        logging_rules: collector.optional_many(command(System::LoggingRule)).await,
        logging_actions: collector.optional_many(command(System::LoggingAction)).await,
        notes: collector.optional_many(command(System::Note)).await,
        ntp_clients: collector.optional_many(command(System::NtpClient)).await,
        ntp_servers: collector.optional_many(command(System::NtpServer)).await,
        scripts: collector.optional_many(command(System::Script)).await,
        script_jobs: collector.optional_many(command(System::ScriptJob)).await,
        schedulers: collector.optional_many(command(System::Scheduler)).await,
        upgrade_mirrors: collector.optional_many(command(System::UpgradeMirror)).await,
        watchdogs: collector.optional_many(command(System::Watchdog)).await,
    })
}

/// Wrap a `/system` command in the top-level print command.
const fn command(command: System) -> PrintCommand {
    PrintCommand::System(command)
}
