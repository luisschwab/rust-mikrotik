//! System API response rows.

use alloc::string::String;
use alloc::vec::Vec;
use core::num::NonZeroU8;

use serde::Deserialize;
use serde::Serialize;

use crate::RouterOsId;
use crate::primitives::interface::InterfaceName;
use crate::primitives::system::RouterOsDate;
use crate::primitives::system::RouterOsDateTime;
use crate::primitives::system::RouterOsDuration;
use crate::primitives::system::RouterOsDurationRange;
use crate::primitives::system::RouterOsTime;
use crate::primitives::system::RouterOsTimeZoneOffset;
use crate::primitives::system::RouterOsVersion;

/// Response row from `/system/identity/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
    /// `RouterOS` system identity.
    pub name: Option<String>,
}

/// Response row from `/system/resource/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Resource {
    /// Router uptime as `RouterOS` reports it, for example `4d17h7m22s`.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub uptime: Option<RouterOsDuration>,
    /// `RouterOS` version string.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub version: Option<RouterOsVersion>,
    /// Build timestamp as local `RouterOS` date/time.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub build_time: Option<RouterOsDateTime>,
    /// Factory software version.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub factory_software: Option<RouterOsVersion>,
    /// Amount of free memory in bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub free_memory: Option<u64>,
    /// Amount of total memory in bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub total_memory: Option<u64>,
    /// CPU name.
    pub cpu: Option<String>,
    /// CPU core count.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub cpu_count: Option<u16>,
    /// Current CPU frequency in MHz.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub cpu_frequency: Option<u32>,
    /// Current CPU load percentage.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub cpu_load: Option<u8>,
    /// Free storage in bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub free_hdd_space: Option<u64>,
    /// Total storage in bytes.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub total_hdd_space: Option<u64>,
    /// Percentage of bad storage blocks, when reported.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub bad_blocks: Option<u8>,
    /// `RouterOS` architecture name.
    pub architecture_name: Option<String>,
    /// Board name.
    pub board_name: Option<String>,
    /// Platform name.
    pub platform: Option<String>,
    /// Sectors written since reboot.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub write_sect_since_reboot: Option<u64>,
    /// Total sectors written.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub write_sect_total: Option<u64>,
}

/// Response row from `/system/routerboard/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Routerboard {
    /// Whether `RouterOS` reports this device as `RouterBOARD` hardware.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub routerboard: Option<bool>,
    /// Board model.
    pub model: Option<String>,
    /// Hardware serial number.
    pub serial_number: Option<String>,
    /// `RouterBOARD` firmware type.
    pub firmware_type: Option<String>,
    /// Factory firmware version.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub factory_firmware: Option<RouterOsVersion>,
    /// Current firmware version.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub current_firmware: Option<RouterOsVersion>,
    /// Upgrade firmware version.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub upgrade_firmware: Option<RouterOsVersion>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Board name, when present.
    pub board_name: Option<String>,
}

/// Response row from `/system/clock/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Clock {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current router calendar date.
    pub date: Option<RouterOsDate>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Time value associated with this entry.
    pub time: Option<RouterOsTime>,
    /// Configured IANA time zone name.
    pub time_zone_name: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current clock offset from GMT.
    pub gmt_offset: Option<RouterOsTimeZoneOffset>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether daylight saving time is currently active.
    pub dst_active: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` automatically detects the time zone.
    pub time_zone_autodetect: Option<bool>,
}

/// Response row from `/system/history/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct HistoryEntry {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Action configured for this row.
    pub action: Option<String>,
    /// User or process that made the recorded configuration change.
    pub by: Option<String>,
    /// Policy names applied to this history entry.
    pub policy: Option<String>,
    /// Redo command text for the history entry.
    pub redo: Option<String>,
    /// Undo command text for the history entry.
    pub undo: Option<String>,
    /// Trace information recorded for this entry.
    pub trace: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Time value associated with this entry.
    pub time: Option<RouterOsDateTime>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the history entry can be undone.
    pub undoable: Option<bool>,
}

/// Response row from `/system/package/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Package {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this package.
    pub name: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// `RouterOS` version associated with this entry.
    pub version: Option<RouterOsVersion>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Build timestamp reported by `RouterOS`.
    pub build_time: Option<RouterOsDateTime>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Package size.
    pub size: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
}

/// Response row from `/system/package/update/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct PackageUpdate {
    /// Update channel selected for package updates.
    pub channel: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Installed `RouterOS` version.
    pub installed_version: Option<RouterOsVersion>,
}

/// Response row from `/system/resource/cpu/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResourceCpu {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// CPU identifier or usage associated with the row.
    pub cpu: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// CPU load percentage.
    pub load: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// IRQ number or IRQ CPU usage counter.
    pub irq: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Disk usage percentage associated with the CPU resource row.
    pub disk: Option<u8>,
}

/// Response row from `/system/logging/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LoggingRule {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Action configured for this row.
    pub action: Option<String>,
    /// Logging topics or topic filters.
    pub topics: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterOS` considers this row invalid.
    pub invalid: Option<bool>,
}

/// Response row from `/system/logging/action/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LoggingAction {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Name of this logging action.
    pub name: Option<String>,
    /// Logging target backend.
    pub target: Option<String>,
    /// Base filename used for disk logging.
    pub disk_file_name: Option<String>,
    /// Remote syslog target address.
    pub remote: Option<String>,
    /// Syslog facility used by the logging action.
    pub syslog_facility: Option<String>,
    /// Syslog severity used by the logging action.
    pub syslog_severity: Option<String>,
    /// Timestamp format used for syslog messages.
    pub syslog_time_format: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of rotated log files kept on disk.
    pub disk_file_count: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of log lines kept per disk log file.
    pub disk_lines_per_file: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Number of log lines retained in memory.
    pub memory_lines: Option<u32>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Port number.
    pub remote_port: Option<u16>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether BSD syslog format is used.
    pub bsd_syslog: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether disk logging stops when storage is full.
    pub disk_stop_on_full: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether memory logging stops when the buffer is full.
    pub memory_stop_on_full: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the logging action persists messages across reboot.
    pub remember: Option<bool>,
}

/// Response row from `/system/ntp/client/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NtpClient {
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
    /// Current NTP client synchronization status.
    pub status: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Measured NTP frequency drift.
    pub freq_drift: Option<i32>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/system/ntp/server/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NtpServer {
    /// Authentication key used by the NTP server.
    pub auth_key: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Stratum advertised when using the local clock.
    pub local_clock_stratum: Option<NonZeroU8>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether broadcast mode is enabled.
    pub broadcast: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether NTP manycast mode is enabled.
    pub manycast: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether multicast mode is enabled.
    pub multicast: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the NTP server uses the router local clock.
    pub use_local_clock: Option<bool>,
}

/// Response row from `/system/routerboard/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RouterboardSettings {
    /// Device selected for `RouterBOOT` startup.
    pub boot_device: Option<String>,
    /// Network boot protocol used by `RouterBOOT`.
    pub boot_protocol: Option<String>,
    /// `RouterBOARD` CPU frequency setting.
    pub cpu_frequency: Option<String>,
    /// Preboot Etherboot mode.
    pub preboot_etherboot: Option<String>,
    /// Server used for preboot Etherboot.
    pub preboot_etherboot_server: Option<String>,
    /// Protected `RouterBOOT` mode.
    pub protected_routerboot: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Hold-button duration required to reformat storage.
    pub reformat_hold_button: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Maximum hold-button duration that triggers storage reformat.
    pub reformat_hold_button_max: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether automatic `RouterBOARD` firmware upgrade is enabled.
    pub auto_upgrade: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterBOOT` is forced to use the backup booter.
    pub force_backup_booter: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether `RouterBOOT` silent boot is enabled.
    pub silent_boot: Option<bool>,
}

/// Response row from `/system/routerboard/reset-button/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct RouterboardResetButton {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Button hold time required for this action.
    pub hold_time: Option<RouterOsDurationRange>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/log/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct LogEntry {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Log message text.
    pub message: Option<String>,
    /// Time value associated with this entry.
    pub time: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Logging topics or topic filters.
    pub topics: Vec<String>,
}

/// Response row from `/system/device-mode/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DeviceMode {
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
}

/// Response row from `/system/leds/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Led {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interface associated with this row.
    pub interface: Option<InterfaceName>,
    /// LEDs controlled by this row.
    pub leds: Option<String>,
    #[serde(rename = "type")]
    /// LED behavior type.
    pub led_type: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this is the default row.
    pub default: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
}

/// Response row from `/system/license/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct License {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// `RouterOS` license level.
    pub nlevel: Option<u8>,
    /// `RouterOS` software ID.
    pub software_id: Option<String>,
}

/// Response row from `/system/note/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Note {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the note is shown at CLI login.
    pub show_at_cli_login: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the note is shown at login.
    pub show_at_login: Option<bool>,
}

/// Response row from `/system/upgrade/mirror/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct UpgradeMirror {
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Interval between upgrade mirror checks.
    pub check_interval: Option<RouterOsDuration>,
    /// Primary upgrade mirror server.
    pub primary_server: Option<String>,
    /// Secondary upgrade mirror server.
    pub secondary_server: Option<String>,
    /// `RouterOS` software ID.
    pub software_id: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
}

/// Response row from `/system/resource/usb/settings/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResourceUsbSettings {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// USB authorization mode.
    pub authorization: Option<bool>,
}

/// Response row from `/system/resource/irq/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResourceIrq {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// IRQ number or IRQ CPU usage counter.
    pub irq: Option<String>,
    /// CPU identifier or usage associated with the row.
    pub cpu: Option<String>,
    /// CPU currently handling this IRQ.
    pub active_cpu: Option<String>,
    /// Users of this IRQ resource.
    pub users: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Counter value for this statistic row.
    pub count: Option<u64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Whether IRQ counters are split per CPU.
    pub per_cpu_count: Option<u64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the entry is read-only.
    pub read_only: Option<bool>,
}

/// Response row from `/system/script/job/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ScriptJob {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    #[serde(rename = ".nextid", deserialize_with = "crate::optional_from_str")]
    /// Next internal `RouterOS` row ID, when reported.
    pub next_id: Option<RouterOsId>,
    /// User that owns the running script job.
    pub owner: Option<String>,
    /// Parent job or object identifier.
    pub parent: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Policy names applied to this script job.
    pub policy: Vec<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Start timestamp for this row.
    pub started: Option<RouterOsDateTime>,
    /// Trace information recorded for this entry.
    pub trace: Option<String>,
    #[serde(rename = "type")]
    /// Script job type.
    pub job_type: Option<String>,
}

/// Response row from `/system/watchdog/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Watchdog {
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether the watchdog timer is enabled.
    pub watchdog_timer: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether watchdog support output files are sent automatically.
    pub auto_send_supout: Option<bool>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether automatic support output generation is enabled.
    pub automatic_supout: Option<bool>,
    /// Address monitored by the watchdog.
    pub watch_address: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Delay before watchdog ping checks start after boot.
    pub ping_start_after_boot: Option<RouterOsDuration>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Timeout used by watchdog ping checks.
    pub ping_timeout: Option<RouterOsDuration>,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;
    use core::time::Duration;

    use super::Resource;
    use crate::Row;
    use crate::primitives::system::RouterOsByteSize;
    use crate::primitives::system::RouterOsDateTime;
    use crate::primitives::system::RouterOsDurationRange;
    use crate::primitives::system::RouterOsTimeZoneOffset;

    #[test]
    fn datetime_parses_current_router_os_format() {
        let datetime = "2024-06-26 11:42:37"
            .parse::<RouterOsDateTime>()
            .expect("current RouterOS datetime should parse");

        assert_eq!(datetime.to_string(), "2024-06-26 11:42:37");
    }

    #[test]
    fn datetime_parses_legacy_router_os_format() {
        let datetime = "Oct/17/2022 10:55:40"
            .parse::<RouterOsDateTime>()
            .expect("legacy RouterOS datetime should parse");

        assert_eq!(datetime.to_string(), "2022-10-17 10:55:40");
    }

    #[test]
    fn resource_deserializes_typed_datetime_and_duration() {
        let mut row = Row::new();
        row.insert("build-time".into(), "Oct/17/2022 10:55:40".into());
        row.insert("uptime".into(), "9w1d1h39m9s".into());
        row.insert("bad-blocks".into(), "0".into());

        let resource = crate::deserialize::<Resource>(&row).expect("resource row should deserialize");

        assert_eq!(
            resource.build_time.expect("build time should be present").to_string(),
            "2022-10-17 10:55:40"
        );
        assert_eq!(
            resource.uptime.expect("uptime should be present").as_duration(),
            Duration::from_secs(9 * 7 * 24 * 60 * 60 + 24 * 60 * 60 + 60 * 60 + 39 * 60 + 9)
        );
        assert_eq!(resource.bad_blocks, Some(0));
    }

    #[test]
    fn datetime_rejects_invalid_values() {
        assert!("2024-13-26 11:42:37".parse::<RouterOsDateTime>().is_err());
        assert!("Oct/32/2022 10:55:40".parse::<RouterOsDateTime>().is_err());
    }

    #[test]
    fn clock_deserializes_date_and_time_types() {
        let mut row = Row::new();
        row.insert("date".into(), "2026-06-10".into());
        row.insert("time".into(), "18:34:08".into());
        row.insert("gmt-offset".into(), "-03:00".into());
        row.insert("dst-active".into(), "false".into());

        let clock = crate::deserialize::<super::Clock>(&row).expect("clock row should deserialize");

        assert_eq!(clock.date.expect("date should be present").to_string(), "2026-06-10");
        assert_eq!(clock.time.expect("time should be present").to_string(), "18:34:08");
        assert_eq!(clock.gmt_offset.expect("offset should be present").minutes(), -180);
        assert_eq!(clock.dst_active, Some(false));
    }

    #[test]
    fn scalar_helpers_parse_ranges_byte_sizes_and_offsets() {
        let range = "0s..1m"
            .parse::<RouterOsDurationRange>()
            .expect("duration range should parse");
        let size = "16k".parse::<RouterOsByteSize>().expect("byte size should parse");
        let offset = "+03:30"
            .parse::<RouterOsTimeZoneOffset>()
            .expect("timezone offset should parse");

        assert_eq!(range.start().to_string(), "0s");
        assert_eq!(range.end().to_string(), "1m");
        assert_eq!(size.bytes(), 16 * 1024);
        assert_eq!(offset.minutes(), 210);
    }
}
