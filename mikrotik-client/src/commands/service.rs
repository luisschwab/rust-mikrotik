//! `RouterOS` service and package-family print command paths.

use core::fmt;

/// `RouterOS` print command `/caps-man/aaa/print`.
const SERVICE_CAPS_MAN_AAA_PRINT: &str = "/caps-man/aaa/print";

/// `RouterOS` print command `/caps-man/manager/print`.
const SERVICE_CAPS_MAN_MANAGER_PRINT: &str = "/caps-man/manager/print";

/// `RouterOS` print command `/caps-man/manager/interface/print`.
const SERVICE_CAPS_MAN_MANAGER_INTERFACE_PRINT: &str = "/caps-man/manager/interface/print";

/// `RouterOS` print command `/certificate/settings/print`.
const SERVICE_CERTIFICATE_SETTINGS_PRINT: &str = "/certificate/settings/print";

/// `RouterOS` print command `/console/settings/print`.
const SERVICE_CONSOLE_SETTINGS_PRINT: &str = "/console/settings/print";

/// `RouterOS` print command `/disk/settings/print`.
const SERVICE_DISK_SETTINGS_PRINT: &str = "/disk/settings/print";

/// `RouterOS` print command `/file/print`.
const SERVICE_FILE_PRINT: &str = "/file/print";

/// `RouterOS` print command `/mpls/settings/print`.
const SERVICE_MPLS_SETTINGS_PRINT: &str = "/mpls/settings/print";

/// `RouterOS` print command `/partitions/print`.
const SERVICE_PARTITION_PRINT: &str = "/partitions/print";

/// `RouterOS` print command `/ppp/aaa/print`.
const SERVICE_PPP_AAA_PRINT: &str = "/ppp/aaa/print";

/// `RouterOS` print command `/ppp/profile/print`.
const SERVICE_PPP_PROFILE_PRINT: &str = "/ppp/profile/print";

/// `RouterOS` print command `/radius/incoming/print`.
const SERVICE_RADIUS_INCOMING_PRINT: &str = "/radius/incoming/print";

/// `RouterOS` print commands in this command family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    /// `RouterOS` print command.
    CapsManAaa,
    /// `RouterOS` print command.
    CapsManManager,
    /// `RouterOS` print command.
    CapsManManagerInterface,
    /// `RouterOS` print command.
    CertificateSettings,
    /// `RouterOS` print command.
    ConsoleSettings,
    /// `RouterOS` print command.
    DiskSettings,
    /// `RouterOS` print command.
    File,
    /// `RouterOS` print command.
    MplsSettings,
    /// `RouterOS` print command.
    Partition,
    /// `RouterOS` print command.
    PppAaa,
    /// `RouterOS` print command.
    PppProfile,
    /// `RouterOS` print command.
    RadiusIncoming,
}

impl Service {
    /// All command variants in generated order.
    pub const ALL: &[Self] = &[
        Self::CapsManAaa,
        Self::CapsManManager,
        Self::CapsManManagerInterface,
        Self::CertificateSettings,
        Self::ConsoleSettings,
        Self::DiskSettings,
        Self::File,
        Self::MplsSettings,
        Self::Partition,
        Self::PppAaa,
        Self::PppProfile,
        Self::RadiusIncoming,
    ];

    /// Return the `RouterOS` API command path.
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::CapsManAaa => SERVICE_CAPS_MAN_AAA_PRINT,
            Self::CapsManManager => SERVICE_CAPS_MAN_MANAGER_PRINT,
            Self::CapsManManagerInterface => SERVICE_CAPS_MAN_MANAGER_INTERFACE_PRINT,
            Self::CertificateSettings => SERVICE_CERTIFICATE_SETTINGS_PRINT,
            Self::ConsoleSettings => SERVICE_CONSOLE_SETTINGS_PRINT,
            Self::DiskSettings => SERVICE_DISK_SETTINGS_PRINT,
            Self::File => SERVICE_FILE_PRINT,
            Self::MplsSettings => SERVICE_MPLS_SETTINGS_PRINT,
            Self::Partition => SERVICE_PARTITION_PRINT,
            Self::PppAaa => SERVICE_PPP_AAA_PRINT,
            Self::PppProfile => SERVICE_PPP_PROFILE_PRINT,
            Self::RadiusIncoming => SERVICE_RADIUS_INCOMING_PRINT,
        }
    }
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_path())
    }
}
