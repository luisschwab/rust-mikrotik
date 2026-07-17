//! `RouterOS` CHR version catalog and image URL helpers.

use core::result::Result as CoreResult;
use core::str::FromStr;
use std::env;

use serde::Deserialize;
use serde::Deserializer;

use crate::error::Error;
use crate::error::Result;

/// CHR image architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChrArch {
    /// `x86_64` CHR image.
    X86_64,
    /// `aarch64` CHR image.
    Aarch64,
}

impl ChrArch {
    /// Return the native host architecture as a CHR image architecture.
    pub(crate) fn host() -> Result<Self> {
        if cfg!(target_arch = "x86_64") {
            Ok(Self::X86_64)
        } else if cfg!(target_arch = "aarch64") {
            Ok(Self::Aarch64)
        } else {
            Err(Error::Tool(format!(
                "unsupported host architecture `{}` for CHR QEMU runner",
                env::consts::ARCH
            )))
        }
    }
}

/// A stable or long-term `RouterOS` release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterOsChannel {
    /// Stable release channel.
    Stable,
    /// Long-term release channel.
    LongTerm,
}

/// One `RouterOS` version supported by the QEMU runner CHR catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum RouterOsVersion {
    /// `RouterOS` 7.23.1.
    #[default]
    V7_23_1,
    /// `RouterOS` 7.23.
    V7_23,
    /// `RouterOS` 7.22.3.
    V7_22_3,
    /// `RouterOS` 7.22.2.
    V7_22_2,
    /// `RouterOS` 7.22.1.
    V7_22_1,
    /// `RouterOS` 7.21.4.
    V7_21_4,
    /// `RouterOS` 7.20.8.
    V7_20_8,
    /// `RouterOS` 7.20.7.
    V7_20_7,
    /// `RouterOS` 6.49.19.
    V6_49_19,
    /// `RouterOS` 6.49.18.
    V6_49_18,
    /// `RouterOS` 6.49.17.
    V6_49_17,
    /// `RouterOS` 6.49.16.
    V6_49_16,
    /// `RouterOS` 6.49.15.
    V6_49_15,
    /// `RouterOS` 6.49.13.
    V6_49_13,
    /// `RouterOS` 6.49.10.
    V6_49_10,
}

impl RouterOsVersion {
    /// Return the `RouterOS` version string as used by `download.mikrotik.com`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V7_23_1 => "7.23.1",
            Self::V7_23 => "7.23",
            Self::V7_22_3 => "7.22.3",
            Self::V7_22_2 => "7.22.2",
            Self::V7_22_1 => "7.22.1",
            Self::V7_21_4 => "7.21.4",
            Self::V7_20_8 => "7.20.8",
            Self::V7_20_7 => "7.20.7",
            Self::V6_49_19 => "6.49.19",
            Self::V6_49_18 => "6.49.18",
            Self::V6_49_17 => "6.49.17",
            Self::V6_49_16 => "6.49.16",
            Self::V6_49_15 => "6.49.15",
            Self::V6_49_13 => "6.49.13",
            Self::V6_49_10 => "6.49.10",
        }
    }

    /// Return stable and/or long-term channels this version belongs to.
    pub const fn channels(self) -> &'static [RouterOsChannel] {
        match self {
            Self::V7_23_1 | Self::V7_23 | Self::V7_22_3 | Self::V7_22_2 | Self::V7_22_1 => &[RouterOsChannel::Stable],
            Self::V7_21_4 | Self::V7_20_8 | Self::V7_20_7 => &[RouterOsChannel::LongTerm],
            Self::V6_49_19 | Self::V6_49_18 => &[RouterOsChannel::Stable, RouterOsChannel::LongTerm],
            Self::V6_49_17 | Self::V6_49_16 | Self::V6_49_15 => &[RouterOsChannel::Stable],
            Self::V6_49_13 | Self::V6_49_10 => &[RouterOsChannel::LongTerm],
        }
    }

    /// Return CHR image architectures available for this version.
    pub const fn image_arches(self) -> &'static [ChrArch] {
        match self {
            Self::V7_23_1
            | Self::V7_23
            | Self::V7_22_3
            | Self::V7_22_2
            | Self::V7_22_1
            | Self::V7_21_4
            | Self::V7_20_8
            | Self::V7_20_7 => &[ChrArch::X86_64, ChrArch::Aarch64],
            Self::V6_49_19
            | Self::V6_49_18
            | Self::V6_49_17
            | Self::V6_49_16
            | Self::V6_49_15
            | Self::V6_49_13
            | Self::V6_49_10 => &[ChrArch::X86_64],
        }
    }
}

impl FromStr for RouterOsVersion {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        ROUTEROS_VERSIONS
            .iter()
            .copied()
            .find(|version| version.as_str() == value)
            .ok_or_else(|| Error::Config(format!("unknown RouterOS version `{value}`")))
    }
}

impl<'de> Deserialize<'de> for RouterOsVersion {
    fn deserialize<D>(deserializer: D) -> CoreResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Default `RouterOS` version for new [`crate::MikrotikDConf`] values.
pub const DEFAULT_ROUTEROS_VERSION: RouterOsVersion = RouterOsVersion::V7_23_1;

/// Stable and long-term CHR versions shown by `MikroTik`'s CHR download page.
pub const ROUTEROS_VERSIONS: &[RouterOsVersion] = &[
    RouterOsVersion::V7_23_1,
    RouterOsVersion::V7_23,
    RouterOsVersion::V7_22_3,
    RouterOsVersion::V7_22_2,
    RouterOsVersion::V7_22_1,
    RouterOsVersion::V7_21_4,
    RouterOsVersion::V7_20_8,
    RouterOsVersion::V7_20_7,
    RouterOsVersion::V6_49_19,
    RouterOsVersion::V6_49_18,
    RouterOsVersion::V6_49_17,
    RouterOsVersion::V6_49_16,
    RouterOsVersion::V6_49_15,
    RouterOsVersion::V6_49_13,
    RouterOsVersion::V6_49_10,
];

/// Choose a guest image architecture for a `RouterOS` version on this host.
pub(crate) fn guest_arch(host_arch: ChrArch, version: RouterOsVersion) -> Result<ChrArch> {
    if version.image_arches().contains(&host_arch) {
        return Ok(host_arch);
    }
    version.image_arches().first().copied().ok_or_else(|| {
        Error::Config(format!(
            "RouterOS version {} has no CHR image architectures configured",
            version.as_str()
        ))
    })
}
