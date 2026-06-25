//! Unit tests for simnet parsing and helpers.

use super::*;
use crate::catalog::chr_url;
use crate::catalog::guest_arch;
use crate::catalog::routeros_version;
use crate::catalog::validate_routeros_versions;
use crate::runner::mac;

#[test]
fn resolves_bundled_topology_file_names() {
    let path = resolve_topology_path(std::path::Path::new("single-router.toml"));

    assert_eq!(
        path,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../topologies")
            .join("single-router.toml")
    );
}

#[test]
fn catalog_contains_stable_and_long_term_v6_and_v7_versions() {
    assert!(routeros_version("7.23.1").is_ok());
    assert!(routeros_version("7.21.4").is_ok());

    let version = routeros_version("6.49.19").unwrap();
    assert!(version.channels.contains(&RouterOsChannel::Stable));
    assert!(version.channels.contains(&RouterOsChannel::LongTerm));
    assert_eq!(version.image_arches, &[ChrArch::X86_64]);
}

#[test]
fn catalog_architectures_match_routeros_major_support() {
    for version in ROUTEROS_VERSIONS {
        let major = version_tuple(version.version).0;
        match major {
            7 => assert_eq!(
                version.image_arches,
                &[ChrArch::X86_64, ChrArch::Aarch64],
                "RouterOS {} should exercise both v7 CHR image architectures",
                version.version
            ),
            6 => assert_eq!(
                version.image_arches,
                &[ChrArch::X86_64],
                "RouterOS {} should use the v6 x86_64 CHR image",
                version.version
            ),
            _ => panic!(
                "RouterOS {} has unsupported major version {major} in the simnet catalog",
                version.version
            ),
        }
    }
}

#[test]
fn catalog_versions_are_unique_and_channel_ordered_newest_first() {
    for (index, version) in ROUTEROS_VERSIONS.iter().enumerate() {
        assert!(
            ROUTEROS_VERSIONS[index + 1..]
                .iter()
                .all(|other| other.version != version.version),
            "duplicate RouterOS version {} in catalog",
            version.version
        );
    }

    for channel in [RouterOsChannel::Stable, RouterOsChannel::LongTerm] {
        let channel_versions = ROUTEROS_VERSIONS
            .iter()
            .filter(|version| version.channels.contains(&channel))
            .map(|version| version.version)
            .collect::<Vec<_>>();

        assert!(
            channel_versions
                .windows(2)
                .all(|window| version_tuple(window[0]) >= version_tuple(window[1])),
            "{channel:?} RouterOS catalog entries should be newest-first: {channel_versions:?}"
        );
    }
}

#[test]
fn rejects_unknown_routeros_version() {
    let topology = Topology::parse(
        r#"
name = "bad-version"
[[routers]]
name = "r1"
version = "7.99.1"
"#,
    )
    .unwrap();

    let error = validate_routeros_versions(&topology).unwrap_err();

    assert!(error.to_string().contains("unknown RouterOS version `7.99.1`"));
}

#[test]
fn topology_parser_applies_defaults() {
    let topology = Topology::parse(
        r#"
name = "defaults"

[[routers]]
name = "r1"
version = "7.23.1"

[[checks]]
type = "all-print-commands"
router = "r1"

[[checks]]
type = "command-rows"
router = "r1"
command = "/system/resource/print"
"#,
    )
    .expect("topology should parse with defaults");

    assert!(!topology.allow_software_emulation);
    assert_eq!(topology.routers[0].memory_mib, 256);
    assert_eq!(topology.routers[0].cpus, 1);
    assert!(matches!(
        topology.checks[0],
        Check::AllPrintCommands {
            allow_unsupported: false,
            ..
        }
    ));
    assert!(matches!(topology.checks[1], Check::CommandRows { min_rows: 1, .. }));
}

#[test]
fn topology_parser_supports_multiline_arrays_and_comments() {
    let topology = Topology::parse(
        r#"
name = "comments"
allow_software_emulation = true # host fallback

[[routers]]
name = "r1"
version = "7.23.1"
bootstrap = [
  "/system/identity/set name=r1", # inline comment
  "/ip/service/enable numbers=api",
]
"#,
    )
    .expect("topology should parse TOML arrays and comments");

    assert!(topology.allow_software_emulation);
    assert_eq!(topology.routers[0].bootstrap.len(), 2);
    assert_eq!(topology.routers[0].bootstrap[0].command, "/system/identity/set");
    assert_eq!(topology.routers[0].bootstrap[0].attributes[0].key, "name");
    assert_eq!(
        topology.routers[0].bootstrap[0].attributes[0].value.as_deref(),
        Some("r1")
    );
}

#[test]
fn topology_parser_rejects_unknown_fields() {
    let error = Topology::parse(
        r#"
name = "bad"
unexpected = true

[[routers]]
name = "r1"
version = "7.23.1"
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("invalid topology TOML"));
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn topology_parser_rejects_missing_required_fields() {
    let error = Topology::parse(
        r#"
name = "missing-router-name"

[[routers]]
version = "7.23.1"
"#,
    )
    .unwrap_err();

    assert!(error.to_string().contains("invalid topology TOML"));
    assert!(error.to_string().contains("missing field"));
}

#[test]
fn builds_chr_raw_image_urls_for_supported_architectures() {
    assert_eq!(
        chr_url("7.23.1", ChrArch::X86_64),
        "https://download.mikrotik.com/routeros/7.23.1/chr-7.23.1.img.zip"
    );
    assert_eq!(
        chr_url("7.23.1", ChrArch::Aarch64),
        "https://download.mikrotik.com/routeros/7.23.1/chr-7.23.1-arm64.img.zip"
    );
    assert_eq!(
        chr_url("6.49.19", ChrArch::X86_64),
        "https://download.mikrotik.com/routeros/6.49.19/chr-6.49.19.img.zip"
    );
}

#[test]
fn selects_x86_guest_for_v6_on_arm64_host() {
    let version = routeros_version("6.49.19").unwrap();

    assert_eq!(guest_arch(ChrArch::Aarch64, version).unwrap(), ChrArch::X86_64);
}

#[test]
fn selects_native_guest_for_v7_when_available() {
    let version = routeros_version("7.23.1").unwrap();

    assert_eq!(guest_arch(ChrArch::Aarch64, version).unwrap(), ChrArch::Aarch64);
    assert_eq!(guest_arch(ChrArch::X86_64, version).unwrap(), ChrArch::X86_64);
}

#[test]
fn all_versions_stress_topology_covers_every_cataloged_version() {
    let topology =
        Topology::parse(include_str!("../../topologies/stress-test.toml")).expect("stress-test topology should parse");
    let versions = topology
        .routers
        .iter()
        .map(|router| router.version.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let catalog_versions = ROUTEROS_VERSIONS
        .iter()
        .map(|version| version.version)
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(versions, catalog_versions);
    assert_eq!(topology.routers.len(), ROUTEROS_VERSIONS.len());
    assert!(
        topology
            .checks
            .iter()
            .filter(|check| matches!(
                check,
                Check::AllPrintCommands {
                    allow_unsupported: true,
                    ..
                }
            ))
            .count()
            >= ROUTEROS_VERSIONS.len()
    );
}

#[test]
fn deterministic_mac_addresses_are_stable() {
    assert_eq!(mac(1, 2), "02:52:00:01:00:02");
}

fn version_tuple(version: &str) -> (u16, u16, u16) {
    let mut parts = version.split('.');
    let major = parse_version_part(version, parts.next());
    let minor = parse_version_part(version, parts.next());
    let patch = parts.next().map_or(0, |part| parse_version_part(version, Some(part)));

    assert!(
        parts.next().is_none(),
        "RouterOS version {version} should have two or three dotted components"
    );

    (major, minor, patch)
}

fn parse_version_part(version: &str, part: Option<&str>) -> u16 {
    part.unwrap_or_else(|| panic!("RouterOS version {version} is missing a dotted component"))
        .parse()
        .unwrap_or_else(|error| panic!("RouterOS version {version} has an invalid component: {error}"))
}
