//! Environment-gated CHR/QEMU integration tests for simnet.

use std::collections::BTreeSet;

use mikrotik_simnet::ChrArch;
use mikrotik_simnet::ROUTEROS_VERSIONS;
use mikrotik_simnet::RouterOsChannel;
use mikrotik_simnet::RouterOsVersion;
use mikrotik_simnet::VersionImage;

#[tokio::test]
async fn single_router_basic_runs_when_enabled() {
    if !mikrotik_simnet::enabled_from_env() {
        println!("skipping simnet test: set MIKROTIK_SIMNET=1 to run QEMU/CHR");
        return;
    }

    mikrotik_simnet::run_topology("single-router.toml")
        .await
        .expect("single-router simnet topology should pass");
}

#[test]
fn version_list_covers_every_cataloged_routeros_version() {
    let plan = mikrotik_simnet::version_list().expect("version list should resolve on supported hosts");

    assert_eq!(plan.selected_versions.len(), ROUTEROS_VERSIONS.len());
    assert_eq!(plan.selected_images, inferred_catalog_images(host_chr_arch()));
    for version in ROUTEROS_VERSIONS {
        assert!(
            plan.selected_versions.contains(&version.version),
            "version list should include RouterOS {}",
            version.version
        );
    }
}

#[test]
fn version_list_infers_architecture_from_host_and_version() {
    let host_arch = host_chr_arch();
    let plan = mikrotik_simnet::version_list().expect("version list should resolve on supported hosts");

    for image in &plan.selected_images {
        let version = ROUTEROS_VERSIONS
            .iter()
            .find(|version| version.version == image.version)
            .expect("listed image should come from catalog");
        assert_eq!(
            image.guest_arch,
            inferred_arch(host_arch, version),
            "RouterOS {} should use inferred architecture",
            image.version
        );
    }
}

#[test]
fn version_list_covers_cataloged_stable_and_long_term_versions() {
    let plan = mikrotik_simnet::version_list().expect("version list should resolve on supported hosts");
    let listed_versions = plan.selected_versions.into_iter().collect::<BTreeSet<_>>();

    let channel_versions = ROUTEROS_VERSIONS
        .iter()
        .filter(|version| {
            version.channels.contains(&RouterOsChannel::Stable) || version.channels.contains(&RouterOsChannel::LongTerm)
        })
        .map(|version| version.version)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        listed_versions, channel_versions,
        "version list should cover every cataloged stable and long-term RouterOS version"
    );
}

/// Return the native CHR architecture for the current test host.
fn host_chr_arch() -> ChrArch {
    if cfg!(target_arch = "x86_64") {
        ChrArch::X86_64
    } else if cfg!(target_arch = "aarch64") {
        ChrArch::Aarch64
    } else {
        panic!(
            "unsupported host architecture `{}` for CHR simnet tests",
            std::env::consts::ARCH
        );
    }
}

/// Infer the CHR image architecture for one cataloged version.
fn inferred_arch(host_arch: ChrArch, version: &RouterOsVersion) -> ChrArch {
    if version.image_arches.contains(&host_arch) {
        host_arch
    } else if version.image_arches.contains(&ChrArch::X86_64) {
        ChrArch::X86_64
    } else {
        version.image_arches[0]
    }
}

/// Return the image row inferred for every cataloged version.
fn inferred_catalog_images(host_arch: ChrArch) -> Vec<VersionImage> {
    ROUTEROS_VERSIONS
        .iter()
        .map(|version| VersionImage {
            version: version.version,
            guest_arch: inferred_arch(host_arch, version),
        })
        .collect()
}
