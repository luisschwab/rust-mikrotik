//! RouterOS CHR version catalog and image URL helpers.

use crate::error::Error;
use crate::error::Result;
use crate::topology::Topology;

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
                "unsupported host architecture `{}` for CHR simnet",
                std::env::consts::ARCH
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

/// One `RouterOS` version supported by the simnet CHR catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterOsVersion {
    /// `RouterOS` version string as used by `download.mikrotik.com`.
    pub version: &'static str,
    /// Stable and/or long-term channels this version belongs to.
    pub channels: &'static [RouterOsChannel],
    /// CHR image architectures available for this version.
    pub image_arches: &'static [ChrArch],
}

/// Stable and long-term CHR versions shown by `MikroTik`'s CHR download page.
pub const ROUTEROS_VERSIONS: &[RouterOsVersion] = &[
    RouterOsVersion {
        version: "7.23.1",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.23",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.22.3",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.22.2",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.22.1",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.21.4",
        channels: &[RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.20.8",
        channels: &[RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "7.20.7",
        channels: &[RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64, ChrArch::Aarch64],
    },
    RouterOsVersion {
        version: "6.49.19",
        channels: &[RouterOsChannel::Stable, RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.18",
        channels: &[RouterOsChannel::Stable, RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.17",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.16",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.15",
        channels: &[RouterOsChannel::Stable],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.13",
        channels: &[RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64],
    },
    RouterOsVersion {
        version: "6.49.10",
        channels: &[RouterOsChannel::LongTerm],
        image_arches: &[ChrArch::X86_64],
    },
];

/// Build the `MikroTik` CHR raw-image archive URL for a `RouterOS` version and architecture.
pub(crate) fn chr_url(version: &str, arch: ChrArch) -> String {
    format!(
        "https://download.mikrotik.com/routeros/{version}/{}",
        chr_archive_filename(version, arch)
    )
}

/// Build the raw-image archive filename for a `RouterOS` version and architecture.
pub(crate) fn chr_archive_filename(version: &str, arch: ChrArch) -> String {
    format!("{}.zip", chr_image_filename(version, arch))
}

/// Build the raw-image filename for a `RouterOS` version and architecture.
pub(crate) fn chr_image_filename(version: &str, arch: ChrArch) -> String {
    match arch {
        ChrArch::X86_64 => format!("chr-{version}.img"),
        ChrArch::Aarch64 => format!("chr-{version}-arm64.img"),
    }
}

/// Look up a cataloged `RouterOS` version.
pub(crate) fn routeros_version(version: &str) -> Result<&'static RouterOsVersion> {
    ROUTEROS_VERSIONS
        .iter()
        .find(|entry| entry.version == version)
        .ok_or_else(|| {
            Error::Manifest(format!(
                "unknown RouterOS version `{version}`; known stable/long-term versions: {}",
                known_routeros_versions()
            ))
        })
}

/// Validate that every topology router uses a cataloged `RouterOS` version.
pub(crate) fn validate_routeros_versions(topology: &Topology) -> Result<()> {
    for router in &topology.routers {
        routeros_version(&router.version)?;
    }
    Ok(())
}

/// Return the cataloged versions in diagnostic order.
pub(crate) fn known_routeros_versions() -> String {
    ROUTEROS_VERSIONS
        .iter()
        .map(|entry| entry.version)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Choose a guest image architecture for a `RouterOS` version on this host.
pub(crate) fn guest_arch(host_arch: ChrArch, version: &RouterOsVersion) -> Result<ChrArch> {
    if version.image_arches.contains(&host_arch) {
        return Ok(host_arch);
    }
    version.image_arches.first().copied().ok_or_else(|| {
        Error::Manifest(format!(
            "RouterOS version {} has no CHR image architectures configured",
            version.version
        ))
    })
}
