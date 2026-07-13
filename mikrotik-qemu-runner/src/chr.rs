//! CHR image cache, download, and archive extraction helpers.

use core::time::Duration;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::thread;

use bitreq::Method;
use bitreq::Request;
use mikrotik_common::info_with_label;
use mikrotik_common::warn_with_label;
use tracing::debug;

use crate::catalog::ChrArch;
use crate::error::Error;
use crate::error::Result;

/// Base URL for `MikroTik` `RouterOS` downloads.
pub const MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL: &str = "https://download.mikrotik.com/routeros/";

/// Directory for cached CHR base images.
pub(crate) const IMAGES_DIR: &str = ".chr-cache/images";

/// Return a cached CHR raw image [`PathBuf`], downloading and unpacking if needed.
pub(crate) fn ensure_chr_image(root: &Path, version: &str, arch: ChrArch) -> Result<PathBuf> {
    let image = root.join(IMAGES_DIR).join(chr_image_filename(version, arch));
    if image.exists() {
        debug!("Using cached CHR {version} {arch:?} image {}", image.display());
        return Ok(image);
    }

    let archive_member = chr_image_filename(version, arch);
    let archive = root.join(IMAGES_DIR).join(chr_archive_filename(version, arch));
    let url = chr_url(version, arch);
    info_with_label!("CHR", "Downloading CHR {version} {arch:?} from {url}");
    download_chr_archive(version, &url, &archive, &archive_member)?;

    debug!("Unpacking {} to {}", archive_member, image.display());
    unpack_chr_archive(&archive, &archive_member, &image)?;
    fs::remove_file(&archive)?;
    Ok(image)
}

/// Build the `MikroTik` CHR raw-image archive URL.
fn chr_url(version: &str, arch: ChrArch) -> String {
    format!(
        "{}{version}/{}",
        MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL,
        chr_archive_filename(version, arch)
    )
}

/// Build the raw-image archive filename.
fn chr_archive_filename(version: &str, arch: ChrArch) -> String {
    format!("{}.zip", chr_image_filename(version, arch))
}

/// Build the raw-image filename.
fn chr_image_filename(version: &str, arch: ChrArch) -> String {
    match arch {
        ChrArch::X86_64 => format!("chr-{version}.img"),
        ChrArch::Aarch64 => format!("chr-{version}-arm64.img"),
    }
}

/// Extract one member from a CHR zip archive into the image cache.
fn unpack_chr_archive(archive_path: &Path, archive_member: &str, image: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| Error::Tool(format!("open CHR archive {}: {error}", archive_path.display())))?;
    let mut member = archive.by_name(archive_member).map_err(|error| {
        Error::Tool(format!(
            "find {archive_member} in CHR archive {}: {error}",
            archive_path.display()
        ))
    })?;
    let mut output = fs::File::create(image)?;
    io::copy(&mut member, &mut output).map_err(|error| {
        Error::Tool(format!(
            "extract {archive_member} from CHR archive {} to {}: {error}",
            archive_path.display(),
            image.display()
        ))
    })?;
    Ok(())
}

/// Download one CHR archive with bounded retries and atomic replacement.
fn download_chr_archive(version: &str, url: &str, archive: &Path, archive_member: &str) -> Result<()> {
    const ATTEMPTS: usize = 5;
    const TIMEOUT_SECONDS: u64 = 180;

    let partial = archive.with_extension("zip.part");
    let mut last_error = None;

    for attempt in 1..=ATTEMPTS {
        if partial.exists() {
            fs::remove_file(&partial)?;
        }

        match try_download_chr_archive(url, &partial, archive_member, TIMEOUT_SECONDS) {
            Ok(()) => {
                fs::rename(&partial, archive)?;
                return Ok(());
            }
            Err(error) => {
                let message = error.to_string();
                last_error = Some(message.clone());
                if attempt < ATTEMPTS {
                    warn_with_label!(
                        "CHR",
                        "Download attempt {attempt}/{ATTEMPTS} for {version} failed: {message}. Retrying..."
                    );
                    thread::sleep(Duration::from_secs(attempt as u64));
                }
            }
        }
    }

    Err(Error::Tool(format!(
        "download CHR {version} from {url}: {}",
        last_error.unwrap_or_else(|| "unknown download error".to_owned())
    )))
}

/// Download one URL to a partial output file.
fn try_download_chr_archive(url: &str, partial: &Path, archive_member: &str, timeout_seconds: u64) -> Result<()> {
    let response = Request::new(Method::Get, url)
        .with_timeout(timeout_seconds)
        .send()
        .map_err(|error| Error::Tool(format!("failed to GET {url}: {error}")))?;

    if !(200..300).contains(&response.status_code) {
        return Err(Error::Tool(format!(
            "request {url}: HTTP {} {}",
            response.status_code, response.reason_phrase
        )));
    }

    let expected_len = content_length(&response)?;
    let body = response.as_bytes();

    if let Some(expected_len) = expected_len {
        if body.len() as u64 != expected_len {
            return Err(Error::Tool(format!(
                "downloaded {} byte(s) from {url}, expected {expected_len}",
                body.len()
            )));
        }
    }

    fs::write(partial, body)?;
    validate_chr_archive(partial, archive_member)?;

    Ok(())
}

/// Return the response `Content-Length`, when present.
fn content_length(response: &bitreq::Response) -> Result<Option<u64>> {
    response
        .headers
        .get("content-length")
        .map(|value| {
            value
                .parse()
                .map_err(|error| Error::Tool(format!("parse Content-Length `{value}`: {error}")))
        })
        .transpose()
}

/// Validate a CHR zip archive before it is promoted into the image cache.
fn validate_chr_archive(archive_path: &Path, archive_member: &str) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| Error::Tool(format!("validate CHR archive {}: {error}", archive_path.display())))?;
    archive.by_name(archive_member).map_err(|error| {
        Error::Tool(format!(
            "validate CHR archive {} contains {archive_member}: {error}",
            archive_path.display()
        ))
    })?;

    Ok(())
}
