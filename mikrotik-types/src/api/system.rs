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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
    /// `RouterOS` system identity.
    pub name: Option<String>,
}

/// Response row from `/system/resource/print`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
    pub bad_blocks: Option<f32>,
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

/// Response row from `/system/health/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Health {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Health sensor name.
    pub name: Option<String>,
    /// Sensor type.
    #[serde(rename = "type")]
    pub health_type: Option<String>,
    /// Current sensor value.
    pub value: Option<String>,
    /// Current health subsystem state.
    pub state: Option<String>,
    /// Health subsystem state expected after reboot.
    pub state_after_reboot: Option<String>,
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
    /// Whether this package is available for activation.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub available: Option<bool>,
    /// Legacy package bundle name.
    pub bundle: Option<String>,
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
    /// Whether update-server certificates are verified.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub check_certificate: Option<bool>,
    /// IP protocol version used for update checks.
    pub ip_version: Option<String>,
    /// Package update operating mode.
    pub mode: Option<String>,
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

/// Response row from `/system/resource/hardware/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ResourceHardware {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Hardware device name.
    pub device: Option<String>,
    /// Device type.
    #[serde(rename = "type")]
    pub hardware_type: Option<String>,
    /// Driver associated with this hardware row.
    pub driver: Option<String>,
    /// IRQ assigned to this hardware row.
    pub irq: Option<String>,
    /// Memory or I/O resource range.
    pub memory: Option<String>,
    /// Hardware device category.
    pub category: Option<String>,
    /// Bus-specific device identifier.
    pub device_id: Option<String>,
    /// Hardware path of the device.
    pub device_path: Option<String>,
    /// Whether this hardware entry is inactive.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub inactive: Option<bool>,
    /// I/O resource range.
    pub io: Option<String>,
    /// Physical or bus location of the device.
    pub location: Option<String>,
    /// Hardware resource name.
    pub name: Option<String>,
    /// Owner or subsystem using the hardware resource.
    pub owner: Option<String>,
    /// Hardware vendor name.
    pub vendor: Option<String>,
    /// Bus-specific vendor identifier.
    pub vendor_id: Option<String>,
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
    /// Whether this logging rule is managed internally by `RouterOS`.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub managed: Option<bool>,
    /// Prefix prepended to messages matched by this rule.
    pub prefix: Option<String>,
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
    /// Whether this logging action is managed internally by `RouterOS`.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub managed: Option<bool>,
    /// Format used for messages sent to a remote logger.
    pub remote_log_format: Option<String>,
    /// Transport protocol used for remote logging.
    pub remote_protocol: Option<String>,
    /// Source address used for remote logging.
    pub src_address: Option<String>,
    /// VRF used to reach the remote logger.
    pub vrf: Option<String>,
}

/// Response row from `/system/ntp/client/print`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct NtpClient {
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
    /// Current NTP client synchronization status.
    pub status: Option<String>,
    /// VRF name.
    pub vrf: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Configured NTP server names or addresses.
    pub servers: Vec<String>,
    /// Server currently used for synchronization.
    pub synced_server: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Measured NTP frequency drift.
    pub freq_drift: Option<f64>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Stratum reported by the synchronized server.
    pub synced_stratum: Option<u8>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Current system clock offset from the synchronized server.
    pub system_offset: Option<f64>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this feature is enabled.
    pub enabled: Option<bool>,
    /// Legacy primary NTP server.
    pub primary_ntp: Option<String>,
    /// Legacy secondary NTP server.
    pub secondary_ntp: Option<String>,
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
    /// Additional structured information attached to the log entry.
    pub extra_info: Option<String>,
}

/// Response row from `/system/device-mode/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct DeviceMode {
    /// Operating mode configured for this entry.
    pub mode: Option<String>,
    /// `RouterOS` versions allowed by device-mode policy.
    pub allowed_versions: Option<String>,
    /// Number of pending or failed device-mode attempts.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub attempt_count: Option<u32>,
    /// Whether bandwidth-test functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub bandwidth_test: Option<bool>,
    /// Whether container functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub container: Option<bool>,
    /// Whether email functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub email: Option<bool>,
    /// Whether fetch functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub fetch: Option<bool>,
    /// Whether the installation has been flagged.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub flagged: Option<bool>,
    /// Whether device-mode flagging is enabled.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub flagging_enabled: Option<bool>,
    /// Whether hotspot functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub hotspot: Option<bool>,
    /// Whether any `RouterOS` version may be installed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub install_any_version: Option<bool>,
    /// Whether `IPsec` functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub ipsec: Option<bool>,
    /// Whether L2TP functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub l2tp: Option<bool>,
    /// Whether partition functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub partitions: Option<bool>,
    /// Whether PPTP functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub pptp: Option<bool>,
    /// Whether proxy functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub proxy: Option<bool>,
    /// Whether `RoMON` functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub romon: Option<bool>,
    /// Whether `RouterBOARD` functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub routerboard: Option<bool>,
    /// Whether scheduler functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub scheduler: Option<bool>,
    /// Whether SMB functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub smb: Option<bool>,
    /// Whether packet-sniffer functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub sniffer: Option<bool>,
    /// Whether SOCKS functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub socks: Option<bool>,
    /// Whether traffic-generator functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub traffic_gen: Option<bool>,
    /// Whether `ZeroTier` functionality is allowed.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub zerotier: Option<bool>,
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
    /// Legacy license level.
    pub level: Option<String>,
    /// System identifier bound to the license.
    pub system_id: Option<String>,
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
    /// Text displayed as the system note.
    pub note: Option<String>,
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
    #[serde(deserialize_with = "crate::comma_list_from_str")]
    /// Counter values split by CPU.
    pub per_cpu_count: Vec<u64>,
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

/// Response row from `/system/script/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Script {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Script name.
    pub name: Option<String>,
    /// Script owner.
    pub owner: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Policy names applied to this script.
    pub policy: Vec<String>,
    /// Last script execution result.
    pub last_started: Option<String>,
    /// Script source text.
    pub source: Option<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is invalid.
    pub invalid: Option<bool>,
    /// Whether the script may run without caller permissions.
    #[serde(deserialize_with = "crate::optional_bool")]
    pub dont_require_permissions: Option<bool>,
    /// Number of times the script has run.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub run_count: Option<u64>,
}

/// Response row from `/system/scheduler/print`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Scheduler {
    #[serde(rename = ".id", deserialize_with = "crate::optional_from_str")]
    /// Internal `RouterOS` row ID.
    pub id: Option<RouterOsId>,
    /// Scheduler entry name.
    pub name: Option<String>,
    /// Scheduler start date.
    pub start_date: Option<String>,
    /// Scheduler start time.
    pub start_time: Option<String>,
    #[serde(deserialize_with = "crate::optional_from_str")]
    /// Scheduler interval.
    pub interval: Option<RouterOsDuration>,
    /// Event script or command.
    pub on_event: Option<String>,
    #[serde(deserialize_with = "crate::comma_list")]
    /// Policy names applied to this scheduler.
    pub policy: Vec<String>,
    #[serde(deserialize_with = "crate::optional_bool")]
    /// Whether this row is disabled.
    pub disabled: Option<bool>,
    /// User that owns the scheduler entry.
    pub owner: Option<String>,
    /// Number of times the scheduler entry has run.
    #[serde(deserialize_with = "crate::optional_from_str")]
    pub run_count: Option<u64>,
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
    use alloc::vec;
    use core::time::Duration;

    use super::NtpClient;
    use super::Package;
    use super::Resource;
    use super::ResourceIrq;
    use crate::RouterOsId;
    use crate::Row;
    use crate::primitives::system::RouterOsByteSize;
    use crate::primitives::system::RouterOsDate;
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
    fn date_and_datetime_parse_lowercase_legacy_router_os_format() {
        let date = "jun/12/2026"
            .parse::<RouterOsDate>()
            .expect("lowercase legacy RouterOS date should parse");
        let datetime = "jun/12/2026 16:33:24"
            .parse::<RouterOsDateTime>()
            .expect("lowercase legacy RouterOS datetime should parse");

        assert_eq!(date.to_string(), "2026-06-12");
        assert_eq!(datetime.to_string(), "2026-06-12 16:33:24");
    }

    #[test]
    fn resource_deserializes_typed_datetime_and_duration() {
        let mut row = Row::new();
        row.insert("build-time".into(), "Oct/17/2022 10:55:40".into());
        row.insert("uptime".into(), "9w1d1h39m9s".into());
        row.insert("bad-blocks".into(), "0.1".into());

        let resource = crate::deserialize::<Resource>(&row).expect("resource row should deserialize");

        assert_eq!(
            resource.build_time.expect("build time should be present").to_string(),
            "2022-10-17 10:55:40"
        );
        assert_eq!(
            resource.uptime.expect("uptime should be present").as_duration(),
            Duration::from_secs(9 * 7 * 24 * 60 * 60 + 24 * 60 * 60 + 60 * 60 + 39 * 60 + 9)
        );
        assert_eq!(resource.bad_blocks, Some(0.1));
    }

    #[test]
    fn resource_irq_deserializes_per_cpu_count_list() {
        let mut row = Row::new();
        row.insert(".id".into(), "*1".into());
        row.insert("active-cpu".into(), "0".into());
        row.insert("count".into(), "6".into());
        row.insert("cpu".into(), "auto".into());
        row.insert("irq".into(), "1".into());
        row.insert(
            "per-cpu-count".into(),
            "0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,6".into(),
        );
        row.insert("read-only".into(), "false".into());
        row.insert("users".into(), "TILEGx_Serial".into());

        let irq = crate::deserialize::<ResourceIrq>(&row).expect("resource IRQ row should deserialize");

        assert_eq!(irq.id.as_ref().map(RouterOsId::as_str), Some("*1"));
        assert_eq!(irq.irq.as_deref(), Some("1"));
        assert_eq!(irq.count, Some(6));
        assert_eq!(irq.per_cpu_count.len(), 36);
        assert_eq!(irq.per_cpu_count[35], 6);
        assert_eq!(irq.per_cpu_count[..35], vec![0; 35]);
        assert_eq!(irq.read_only, Some(false));
    }

    #[test]
    fn ntp_client_deserializes_decimal_drift_and_offset() {
        let mut row = Row::new();
        row.insert("enabled".into(), "true".into());
        row.insert("freq-drift".into(), "8.003".into());
        row.insert("mode".into(), "unicast".into());
        row.insert("servers".into(), "201.49.148.135,200.160.7.186".into());
        row.insert("status".into(), "synchronized".into());
        row.insert("synced-server".into(), "200.160.7.186".into());
        row.insert("synced-stratum".into(), "1".into());
        row.insert("system-offset".into(), "-0.025".into());
        row.insert("vrf".into(), "main".into());

        let client = crate::deserialize::<NtpClient>(&row).expect("NTP client row should deserialize");

        assert_eq!(client.enabled, Some(true));
        assert_eq!(client.freq_drift, Some(8.003));
        assert_eq!(client.system_offset, Some(-0.025));
        assert_eq!(client.synced_stratum, Some(1));
        assert_eq!(client.servers.len(), 2);
        assert_eq!(client.synced_server.as_deref(), Some("200.160.7.186"));
    }

    #[test]
    fn datetime_rejects_invalid_values() {
        assert!("2024-13-26 11:42:37".parse::<RouterOsDateTime>().is_err());
        assert!("Oct/32/2022 10:55:40".parse::<RouterOsDateTime>().is_err());
    }

    #[test]
    fn persisted_response_models_remain_permissive_for_historical_fields() {
        let identity = serde_json::from_str::<super::Identity>(r#"{"name":"edge","historical-field":"value"}"#)
            .expect("persisted response rows should tolerate fields unknown to this build");

        assert_eq!(identity.name.as_deref(), Some("edge"));
    }

    #[test]
    fn legacy_package_bundle_is_a_package_name() {
        let row = Row::from([("bundle".to_string(), "routeros-x86".to_string())]);

        let package = crate::deserialize::<Package>(&row).expect("legacy package bundle should deserialize");

        assert_eq!(package.bundle.as_deref(), Some("routeros-x86"));
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
