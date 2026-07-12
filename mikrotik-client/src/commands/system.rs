//! `RouterOS` system print command paths.

/// `RouterOS` print command `/system/clock/print`.
const SYSTEM_CLOCK_PRINT: &str = "/system/clock/print";

/// `RouterOS` print command `/system/device-mode/print`.
const SYSTEM_DEVICE_MODE_PRINT: &str = "/system/device-mode/print";

/// `RouterOS` print command `/system/history/print`.
const SYSTEM_HISTORY_ENTRY_PRINT: &str = "/system/history/print";

/// `RouterOS` print command `/system/health/print`.
const SYSTEM_HEALTH_PRINT: &str = "/system/health/print";

/// `RouterOS` print command `/system/identity/print`.
const SYSTEM_IDENTITY_PRINT: &str = "/system/identity/print";

/// `RouterOS` print command `/system/leds/print`.
const SYSTEM_LED_PRINT: &str = "/system/leds/print";

/// `RouterOS` print command `/system/license/print`.
const SYSTEM_LICENSE_PRINT: &str = "/system/license/print";

/// `RouterOS` print command `/log/print`.
const SYSTEM_LOG_ENTRY_PRINT: &str = "/log/print";

/// `RouterOS` print command `/system/logging/action/print`.
const SYSTEM_LOGGING_ACTION_PRINT: &str = "/system/logging/action/print";

/// `RouterOS` print command `/system/logging/print`.
const SYSTEM_LOGGING_RULE_PRINT: &str = "/system/logging/print";

/// `RouterOS` print command `/system/note/print`.
const SYSTEM_NOTE_PRINT: &str = "/system/note/print";

/// `RouterOS` print command `/system/ntp/client/print`.
const SYSTEM_NTP_CLIENT_PRINT: &str = "/system/ntp/client/print";

/// `RouterOS` print command `/system/ntp/server/print`.
const SYSTEM_NTP_SERVER_PRINT: &str = "/system/ntp/server/print";

/// `RouterOS` print command `/system/package/print`.
const SYSTEM_PACKAGE_PRINT: &str = "/system/package/print";

/// `RouterOS` print command `/system/package/update/print`.
const SYSTEM_PACKAGE_UPDATE_PRINT: &str = "/system/package/update/print";

/// `RouterOS` print command `/system/resource/print`.
const SYSTEM_RESOURCE_PRINT: &str = "/system/resource/print";

/// `RouterOS` print command `/system/resource/cpu/print`.
const SYSTEM_RESOURCE_CPU_PRINT: &str = "/system/resource/cpu/print";

/// `RouterOS` print command `/system/resource/irq/print`.
const SYSTEM_RESOURCE_IRQ_PRINT: &str = "/system/resource/irq/print";

/// `RouterOS` print command `/system/resource/hardware/print`.
const SYSTEM_RESOURCE_HARDWARE_PRINT: &str = "/system/resource/hardware/print";

/// `RouterOS` print command `/system/resource/usb/settings/print`.
const SYSTEM_RESOURCE_USB_SETTINGS_PRINT: &str = "/system/resource/usb/settings/print";

/// `RouterOS` print command `/system/routerboard/print`.
const SYSTEM_ROUTERBOARD_PRINT: &str = "/system/routerboard/print";

/// `RouterOS` print command `/system/routerboard/reset-button/print`.
const SYSTEM_ROUTERBOARD_RESET_BUTTON_PRINT: &str = "/system/routerboard/reset-button/print";

/// `RouterOS` print command `/system/routerboard/settings/print`.
const SYSTEM_ROUTERBOARD_SETTINGS_PRINT: &str = "/system/routerboard/settings/print";

/// `RouterOS` print command `/system/script/job/print`.
const SYSTEM_SCRIPT_JOB_PRINT: &str = "/system/script/job/print";

/// `RouterOS` print command `/system/script/print`.
const SYSTEM_SCRIPT_PRINT: &str = "/system/script/print";

/// `RouterOS` print command `/system/scheduler/print`.
const SYSTEM_SCHEDULER_PRINT: &str = "/system/scheduler/print";

/// `RouterOS` print command `/system/upgrade/mirror/print`.
const SYSTEM_UPGRADE_MIRROR_PRINT: &str = "/system/upgrade/mirror/print";

/// `RouterOS` print command `/system/watchdog/print`.
const SYSTEM_WATCHDOG_PRINT: &str = "/system/watchdog/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum System {
    /// `RouterOS` print command.
    Clock,
    /// `RouterOS` print command.
    DeviceMode,
    /// `RouterOS` print command.
    HistoryEntry,
    /// `RouterOS` print command.
    Health,
    /// `RouterOS` print command.
    Identity,
    /// `RouterOS` print command.
    Led,
    /// `RouterOS` print command.
    License,
    /// `RouterOS` print command.
    LogEntry,
    /// `RouterOS` print command.
    LoggingAction,
    /// `RouterOS` print command.
    LoggingRule,
    /// `RouterOS` print command.
    Note,
    /// `RouterOS` print command.
    NtpClient,
    /// `RouterOS` print command.
    NtpServer,
    /// `RouterOS` print command.
    Package,
    /// `RouterOS` print command.
    PackageUpdate,
    /// `RouterOS` print command.
    Resource,
    /// `RouterOS` print command.
    ResourceCpu,
    /// `RouterOS` print command.
    ResourceHardware,
    /// `RouterOS` print command.
    ResourceIrq,
    /// `RouterOS` print command.
    ResourceUsbSettings,
    /// `RouterOS` print command.
    Routerboard,
    /// `RouterOS` print command.
    RouterboardResetButton,
    /// `RouterOS` print command.
    RouterboardSettings,
    /// `RouterOS` print command.
    Script,
    /// `RouterOS` print command.
    ScriptJob,
    /// `RouterOS` print command.
    Scheduler,
    /// `RouterOS` print command.
    UpgradeMirror,
    /// `RouterOS` print command.
    Watchdog,
}

impl System {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::Clock,
        Self::DeviceMode,
        Self::Health,
        Self::HistoryEntry,
        Self::Identity,
        Self::Led,
        Self::License,
        Self::LogEntry,
        Self::LoggingAction,
        Self::LoggingRule,
        Self::Note,
        Self::NtpClient,
        Self::NtpServer,
        Self::Package,
        Self::PackageUpdate,
        Self::Resource,
        Self::ResourceCpu,
        Self::ResourceHardware,
        Self::ResourceIrq,
        Self::ResourceUsbSettings,
        Self::Routerboard,
        Self::RouterboardResetButton,
        Self::RouterboardSettings,
        Self::Script,
        Self::ScriptJob,
        Self::Scheduler,
        Self::UpgradeMirror,
        Self::Watchdog,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::Clock => SYSTEM_CLOCK_PRINT,
            Self::DeviceMode => SYSTEM_DEVICE_MODE_PRINT,
            Self::Health => SYSTEM_HEALTH_PRINT,
            Self::HistoryEntry => SYSTEM_HISTORY_ENTRY_PRINT,
            Self::Identity => SYSTEM_IDENTITY_PRINT,
            Self::Led => SYSTEM_LED_PRINT,
            Self::License => SYSTEM_LICENSE_PRINT,
            Self::LogEntry => SYSTEM_LOG_ENTRY_PRINT,
            Self::LoggingAction => SYSTEM_LOGGING_ACTION_PRINT,
            Self::LoggingRule => SYSTEM_LOGGING_RULE_PRINT,
            Self::Note => SYSTEM_NOTE_PRINT,
            Self::NtpClient => SYSTEM_NTP_CLIENT_PRINT,
            Self::NtpServer => SYSTEM_NTP_SERVER_PRINT,
            Self::Package => SYSTEM_PACKAGE_PRINT,
            Self::PackageUpdate => SYSTEM_PACKAGE_UPDATE_PRINT,
            Self::Resource => SYSTEM_RESOURCE_PRINT,
            Self::ResourceCpu => SYSTEM_RESOURCE_CPU_PRINT,
            Self::ResourceHardware => SYSTEM_RESOURCE_HARDWARE_PRINT,
            Self::ResourceIrq => SYSTEM_RESOURCE_IRQ_PRINT,
            Self::ResourceUsbSettings => SYSTEM_RESOURCE_USB_SETTINGS_PRINT,
            Self::Routerboard => SYSTEM_ROUTERBOARD_PRINT,
            Self::RouterboardResetButton => SYSTEM_ROUTERBOARD_RESET_BUTTON_PRINT,
            Self::RouterboardSettings => SYSTEM_ROUTERBOARD_SETTINGS_PRINT,
            Self::Script => SYSTEM_SCRIPT_PRINT,
            Self::ScriptJob => SYSTEM_SCRIPT_JOB_PRINT,
            Self::Scheduler => SYSTEM_SCHEDULER_PRINT,
            Self::UpgradeMirror => SYSTEM_UPGRADE_MIRROR_PRINT,
            Self::Watchdog => SYSTEM_WATCHDOG_PRINT,
        }
    }
}

mikrotik_common::impl_command_display!(System);
