//! RouterOS catalog listing for CHR simulation.

use crate::catalog::ChrArch;
use crate::catalog::ROUTEROS_VERSIONS;
use crate::catalog::guest_arch;
use crate::error::Result;

/// One `RouterOS` CHR image inferred for one cataloged version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionImage {
    /// `RouterOS` version string.
    pub version: &'static str,
    /// CHR image architecture inferred for this host.
    pub guest_arch: ChrArch,
}

/// Resolved `RouterOS` versions and images for this host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionList {
    /// CHR image rows inferred for this host.
    pub selected_images: Vec<VersionImage>,
    /// Versions represented by `selected_images`.
    pub selected_versions: Vec<&'static str>,
}

/// Resolve the `RouterOS` versions and CHR images inferred for this host.
///
/// # Errors
///
/// Returns an error when the current host architecture is not supported by the
/// CHR simulator.
pub fn version_list() -> Result<VersionList> {
    version_list_for_host(ChrArch::host()?)
}

/// Resolve the `RouterOS` versions and CHR images inferred for a host architecture.
fn version_list_for_host(host_arch: ChrArch) -> Result<VersionList> {
    let mut selected_images = Vec::new();
    let mut selected_versions = Vec::new();

    for version in ROUTEROS_VERSIONS {
        let selected_arch = guest_arch(host_arch, version)?;
        selected_images.push(VersionImage {
            version: version.version,
            guest_arch: selected_arch,
        });
        selected_versions.push(version.version);
    }

    Ok(VersionList {
        selected_images,
        selected_versions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_host_version_list_selects_every_cataloged_routeros_version() {
        let plan = version_list_for_host(ChrArch::X86_64).expect("x86 host version list should resolve");

        assert_eq!(plan.selected_versions, catalog_versions_matching(|_| true));
        assert_eq!(
            plan.selected_images,
            catalog_images_matching(|_, arch| arch == ChrArch::X86_64)
        );
    }

    #[test]
    fn arm64_host_version_list_selects_every_cataloged_routeros_version_with_inferred_architecture() {
        let plan = version_list_for_host(ChrArch::Aarch64).expect("arm64 host version list should resolve");

        assert_eq!(plan.selected_versions, catalog_versions_matching(|_| true));
        assert_eq!(
            plan.selected_images,
            ROUTEROS_VERSIONS
                .iter()
                .map(|version| VersionImage {
                    version: version.version,
                    guest_arch: guest_arch(ChrArch::Aarch64, version)
                        .expect("catalog version should infer a guest arch"),
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn arm64_host_version_list_uses_x86_for_routeros_six_and_arm64_for_routeros_seven() {
        let plan = version_list_for_host(ChrArch::Aarch64).expect("arm64 host version list should resolve");

        for image in plan.selected_images {
            if image.version.starts_with('6') {
                assert_eq!(image.guest_arch, ChrArch::X86_64);
            } else {
                assert_eq!(image.guest_arch, ChrArch::Aarch64);
            }
        }
    }

    fn catalog_versions_matching(predicate: impl Fn(&crate::catalog::RouterOsVersion) -> bool) -> Vec<&'static str> {
        ROUTEROS_VERSIONS
            .iter()
            .filter(|version| predicate(version))
            .map(|version| version.version)
            .collect()
    }

    fn catalog_images_matching(
        predicate: impl Fn(&crate::catalog::RouterOsVersion, ChrArch) -> bool,
    ) -> Vec<VersionImage> {
        let mut images = Vec::new();
        for version in ROUTEROS_VERSIONS {
            for arch in version.image_arches {
                if predicate(version, *arch) {
                    images.push(VersionImage {
                        version: version.version,
                        guest_arch: *arch,
                    });
                }
            }
        }
        images
    }
}
