//! Validate the typed CHR catalog against `MikroTik`'s live release metadata and archives.

use mikrotik_qemu_runner::ChrArch;
use mikrotik_qemu_runner::Error;
use mikrotik_qemu_runner::MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL;
use mikrotik_qemu_runner::ROUTEROS_VERSIONS;
use mikrotik_qemu_runner::Result;
use mikrotik_qemu_runner::RouterOsChannel;
use mikrotik_qemu_runner::RouterOsVersion;
use xshell::Shell;
use xshell::cmd;

/// Base URL for `MikroTik`'s current-version metadata.
const MIKROTIK_ROUTEROS_UPGRADE_BASE_URL: &str = "https://upgrade.mikrotik.com/routeros";

/// Live release-channel endpoints and the catalog properties their responses must have.
const CHANNEL_HEADS: &[(&str, &str, &str, RouterOsChannel)] = &[
    ("RouterOS v7 stable", "NEWESTa7.stable", "7.", RouterOsChannel::Stable),
    (
        "RouterOS v7 long-term",
        "NEWESTa7.long-term",
        "7.",
        RouterOsChannel::LongTerm,
    ),
    ("RouterOS v6 stable", "NEWEST6.stable", "6.", RouterOsChannel::Stable),
    (
        "RouterOS v6 long-term",
        "NEWEST6.long-term",
        "6.",
        RouterOsChannel::LongTerm,
    ),
];

/// Check live channel heads and every archive represented by the typed catalog.
fn main() -> Result<()> {
    let shell = Shell::new()?;
    check_channel_heads(&shell)?;
    check_images(&shell)?;
    Ok(())
}

/// Ensure every current stable and long-term channel head is classified in the catalog.
fn check_channel_heads(shell: &Shell) -> Result<()> {
    for &(label, endpoint, version_prefix, channel) in CHANNEL_HEADS {
        let url = format!("{MIKROTIK_ROUTEROS_UPGRADE_BASE_URL}/{endpoint}");
        let response = cmd!(
            shell,
            "curl --fail --silent --show-error --location --max-time 30 {url}"
        )
        .read()?;
        let version_text = response
            .split_ascii_whitespace()
            .next()
            .ok_or_else(|| Error::Tool(format!("{label} returned an empty response")))?;
        if !version_text.starts_with(version_prefix) {
            return Err(Error::Tool(format!(
                "{label} returned version `{version_text}` outside the expected `{version_prefix}` series"
            )));
        }

        let version = version_text.parse::<RouterOsVersion>().map_err(|error| {
            Error::Tool(format!(
                "{label} head {version_text} is missing from the typed CHR catalog: {error}"
            ))
        })?;
        if !version.channels().contains(&channel) {
            return Err(Error::Tool(format!(
                "{label} head {version_text} is not classified as {channel:?}"
            )));
        }

        println!("{label} head is cataloged: {version_text}");
    }
    Ok(())
}

/// Ensure each architecture declared for every catalog version has an official CHR archive.
fn check_images(shell: &Shell) -> Result<()> {
    let mut image_count = 0;
    for &version in ROUTEROS_VERSIONS {
        for &arch in version.image_arches() {
            let url = chr_archive_url(version, arch);
            cmd!(
                shell,
                "curl --fail --silent --show-error --location --head --max-time 30 --output /dev/null {url}"
            )
            .run()
            .map_err(|error| Error::Tool(format!("CHR image is unavailable at {url}: {error}")))?;
            println!("CHR image is available: {url}");
            image_count += 1;
        }
    }
    println!("Validated {image_count} CHR image archives.");
    Ok(())
}

/// Build an official raw CHR image archive URL for one version and architecture.
fn chr_archive_url(version: RouterOsVersion, arch: ChrArch) -> String {
    let version = version.as_str();
    let architecture_suffix = match arch {
        ChrArch::X86_64 => "",
        ChrArch::Aarch64 => "-arm64",
    };
    format!("{MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL}{version}/chr-{version}{architecture_suffix}.img.zip")
}
