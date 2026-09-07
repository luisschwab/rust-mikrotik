//! CHR image cache, HTTP download, validation, and archive extraction.

use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use bitreq::Method;
use bitreq::Request;
use mikrotik_common::info_with_label;
use mikrotik_common::warn_with_label;
use tracing::debug;

use super::progress::DownloadProgress;
use super::progress::DownloadProgressEntry;
use super::progress::DownloadProgressGroupHandle;
use crate::catalog::ChrArch;
use crate::error::Error;
use crate::error::Result;

/// Base URL for `MikroTik` `RouterOS` downloads.
pub const MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL: &str = "https://download.mikrotik.com/routeros/";

/// Directory for cached CHR base images.
pub(crate) const IMAGES_DIR: &str = ".chr-cache/images";

/// Maximum duration of one CHR archive request.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);

/// Return a cached CHR raw image [`PathBuf`], downloading and unpacking if needed.
pub(crate) fn ensure_chr_image(
    root: &Path,
    version: &str,
    arch: ChrArch,
    progress: Option<&DownloadProgressGroupHandle>,
) -> Result<PathBuf> {
    let image = root.join(IMAGES_DIR).join(chr_image_filename(version, arch));
    if image.exists() {
        debug!("Using cached CHR {version} {arch:?} image {}", image.display());
        return Ok(image);
    }

    let archive_member = chr_image_filename(version, arch);
    let archive = root.join(IMAGES_DIR).join(chr_archive_filename(version, arch));
    let url = chr_url(version, arch);
    if !progress.is_some_and(DownloadProgressGroupHandle::is_interactive) {
        info_with_label!("CHR", "Downloading CHR {version} {arch:?} from {url}");
    }
    download_chr_archive(version, &url, &archive, &archive_member, progress)?;

    debug!("Unpacking {} to {}", archive_member, image.display());
    unpack_chr_archive(&archive, &archive_member, &image)?;
    fs::remove_file(&archive).map_err(|source| Error::io("remove downloaded CHR archive", &archive, source))?;
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
pub(crate) fn chr_archive_filename(version: &str, arch: ChrArch) -> String {
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
    let file = fs::File::open(archive_path).map_err(|source| Error::io("open CHR archive", archive_path, source))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| Error::Tool(format!("open CHR archive {}: {error}", archive_path.display())))?;
    let mut member = archive.by_name(archive_member).map_err(|error| {
        Error::Tool(format!(
            "find {archive_member} in CHR archive {}: {error}",
            archive_path.display()
        ))
    })?;
    let mut output = fs::File::create(image).map_err(|source| Error::io("create CHR image", image, source))?;
    io::copy(&mut member, &mut output).map_err(|source| Error::io("extract CHR archive member to", image, source))?;
    Ok(())
}

/// Download one CHR archive with bounded retries and atomic replacement.
fn download_chr_archive(
    version: &str,
    url: &str,
    archive: &Path,
    archive_member: &str,
    progress_group: Option<&DownloadProgressGroupHandle>,
) -> Result<()> {
    const ATTEMPTS: usize = 5;

    let partial = archive.with_extension("zip.part");
    let archive_filename = format!("{archive_member}.zip");
    let mut progress_entry = progress_group.and_then(|group| group.add(&archive_filename));
    let mut last_error = None;

    for attempt in 1..=ATTEMPTS {
        if partial.exists() && validate_chr_archive(&partial, archive_member).is_ok() {
            fs::rename(&partial, archive)
                .map_err(|source| Error::io("promote completed partial CHR archive to", archive, source))?;
            if let Some(progress_entry) = &mut progress_entry {
                progress_entry.complete();
            }
            return Ok(());
        }

        match try_download_chr_archive(
            url,
            &partial,
            archive_member,
            DOWNLOAD_TIMEOUT,
            progress_group.is_some(),
            progress_entry.as_ref(),
        ) {
            Ok(()) => {
                fs::rename(&partial, archive)
                    .map_err(|source| Error::io("promote partial CHR archive to", archive, source))?;
                if let Some(progress_entry) = &mut progress_entry {
                    progress_entry.complete();
                }
                return Ok(());
            }
            Err(error) => {
                let message = error.to_string();
                last_error = Some(message.clone());
                if attempt < ATTEMPTS {
                    let warning =
                        format!("Download attempt {attempt}/{ATTEMPTS} for {version} failed: {message}. Retrying...");
                    if let Some(progress_entry) = &progress_entry {
                        progress_entry.println(format!("CHR: {warning}"));
                    } else {
                        warn_with_label!("CHR", "{warning}");
                    }
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
fn try_download_chr_archive(
    url: &str,
    partial: &Path,
    archive_member: &str,
    timeout: Duration,
    grouped_output: bool,
    progress_entry: Option<&DownloadProgressEntry>,
) -> Result<()> {
    let resume_from = match fs::metadata(partial) {
        Ok(metadata) => metadata.len(),
        Err(source) if source.kind() == io::ErrorKind::NotFound => 0,
        Err(source) => return Err(Error::io("inspect partial CHR archive", partial, source)),
    };
    let mut request = Request::new(Method::Get, url).with_timeout(timeout.as_secs());
    if resume_from > 0 {
        request = request.with_header("Range", format!("bytes={resume_from}-"));
    }
    let response = request
        .send_lazy()
        .map_err(|error| Error::Tool(format!("failed to GET {url}: {error}")))?;

    let (downloaded, expected_len, append) = if response.status_code == 206 {
        let expected_len = content_range_total(&response, resume_from)?;
        (resume_from, Some(expected_len), true)
    } else if (200..300).contains(&response.status_code) {
        // A server may ignore Range and return the complete archive. Replace the
        // partial file in that case so old bytes are not duplicated.
        (0, content_length(&response)?, false)
    } else {
        return Err(Error::Tool(format!(
            "request {url}: HTTP {} {}",
            response.status_code, response.reason_phrase
        )));
    };

    let output = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(partial)
        .map_err(|source| Error::io("open partial CHR archive", partial, source))?;
    let archive_filename = format!("{archive_member}.zip");
    let mut progress = DownloadProgress::new(
        response,
        expected_len,
        downloaded,
        &archive_filename,
        grouped_output,
        progress_entry,
    );
    let response_len = io::copy(&mut progress, &mut io::BufWriter::new(output))
        .map_err(|source| Error::io("write partial CHR archive", partial, source))?;
    progress.finish();
    let actual_len = downloaded.saturating_add(response_len);

    if let Some(expected_len) = expected_len {
        if actual_len != expected_len {
            return Err(Error::Tool(format!(
                "downloaded {actual_len} byte(s) from {url}, expected {expected_len}"
            )));
        }
    }

    validate_chr_archive(partial, archive_member)?;

    Ok(())
}

/// Return and validate the total length from a partial response's `Content-Range`.
fn content_range_total(response: &bitreq::ResponseLazy, resume_from: u64) -> Result<u64> {
    let value = response
        .headers
        .get("content-range")
        .ok_or_else(|| Error::Tool("partial CHR response is missing Content-Range".to_owned()))?;
    let range = value.strip_prefix("bytes ").ok_or_else(|| {
        Error::Tool(format!(
            "parse Content-Range `{value}`: expected `bytes start-end/total`"
        ))
    })?;
    let (bounds, total) = range
        .split_once('/')
        .ok_or_else(|| Error::Tool(format!("parse Content-Range `{value}`: missing total length")))?;
    let (start, end) = bounds
        .split_once('-')
        .ok_or_else(|| Error::Tool(format!("parse Content-Range `{value}`: missing byte bounds")))?;
    let start = start
        .parse::<u64>()
        .map_err(|error| Error::Tool(format!("parse Content-Range start `{start}`: {error}")))?;
    let end = end
        .parse::<u64>()
        .map_err(|error| Error::Tool(format!("parse Content-Range end `{end}`: {error}")))?;
    let total = total
        .parse::<u64>()
        .map_err(|error| Error::Tool(format!("parse Content-Range total `{total}`: {error}")))?;

    if start != resume_from || end < start || end >= total {
        return Err(Error::Tool(format!(
            "Content-Range `{value}` does not continue partial CHR archive at byte {resume_from}"
        )));
    }

    Ok(total)
}

/// Return the response `Content-Length`, when present.
fn content_length(response: &bitreq::ResponseLazy) -> Result<Option<u64>> {
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
    let file = fs::File::open(archive_path)
        .map_err(|source| Error::io("open CHR archive for validation", archive_path, source))?;
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

#[cfg(test)]
mod tests {
    use std::env;
    use std::io::Cursor;
    use std::io::Read as _;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::process;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        env::temp_dir().join(format!("mikrotik-chr-test-{}-{name}", process::id()))
    }

    fn serve_once(status: &str, headers: &str, body: Vec<u8>) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_owned();
        let headers = headers.to_owned();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let request_len = stream.read(&mut request).unwrap();
            write!(stream, "HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n").unwrap();
            stream.write_all(&body).unwrap();
            String::from_utf8_lossy(&request[..request_len]).into_owned()
        });
        (format!("http://{address}/chr.zip"), server)
    }

    fn chr_archive_body(archive_member: &str) -> Vec<u8> {
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file(archive_member, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"router image").unwrap();
        archive.finish().unwrap().into_inner()
    }

    #[test]
    fn image_names_and_urls_follow_mikrotik_download_conventions() {
        assert_eq!(chr_image_filename("7.23.1", ChrArch::X86_64), "chr-7.23.1.img");
        assert_eq!(chr_image_filename("7.23.1", ChrArch::Aarch64), "chr-7.23.1-arm64.img");
        assert_eq!(chr_archive_filename("7.23.1", ChrArch::X86_64), "chr-7.23.1.img.zip");
        assert_eq!(
            chr_url("7.23.1", ChrArch::Aarch64),
            "https://download.mikrotik.com/routeros/7.23.1/chr-7.23.1-arm64.img.zip"
        );
    }

    #[test]
    fn existing_cached_image_is_returned_without_downloading() {
        let root = temporary_root("cached");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let image = root.join(IMAGES_DIR).join("chr-7.23.1.img");
        fs::create_dir_all(image.parent().unwrap()).unwrap();
        fs::write(&image, b"cached image").unwrap();

        assert_eq!(ensure_chr_image(&root, "7.23.1", ChrArch::X86_64, None).unwrap(), image);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn archive_validation_and_extraction_require_the_named_member() {
        let root = temporary_root("extract");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("chr.zip");
        let image_path = root.join("chr.img");
        let file = fs::File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("chr-7.23.1.img", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"router image").unwrap();
        archive.finish().unwrap();

        validate_chr_archive(&archive_path, "chr-7.23.1.img").unwrap();
        assert!(validate_chr_archive(&archive_path, "missing.img").is_err());
        unpack_chr_archive(&archive_path, "chr-7.23.1.img", &image_path).unwrap();
        assert_eq!(fs::read(&image_path).unwrap(), b"router image");
        assert!(unpack_chr_archive(&archive_path, "missing.img", &image_path).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_and_missing_archives_report_validation_errors() {
        let root = temporary_root("invalid");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let invalid = root.join("invalid.zip");
        fs::write(&invalid, b"not a zip archive").unwrap();

        assert!(validate_chr_archive(&root.join("missing.zip"), "chr.img").is_err());
        assert!(validate_chr_archive(&invalid, "chr.img").is_err());
        assert!(unpack_chr_archive(&root.join("missing.zip"), "chr.img", &root.join("image")).is_err());
        assert!(unpack_chr_archive(&invalid, "chr.img", &root.join("image")).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn local_http_download_validates_status_length_and_zip_contents() {
        let root = temporary_root("download");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let body = chr_archive_body("chr-7.23.1.img");

        let (url, server) = serve_once("200 OK", &format!("Content-Length: {}\r\n", body.len()), body.clone());
        let partial = root.join("download.part");
        try_download_chr_archive(&url, &partial, "chr-7.23.1.img", Duration::from_secs(2), false, None).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(&partial).unwrap(), body);

        let (url, server) = serve_once("404 Not Found", "Content-Length: 0\r\n", Vec::new());
        assert!(
            try_download_chr_archive(&url, &partial, "chr-7.23.1.img", Duration::from_secs(2), false, None).is_err()
        );
        server.join().unwrap();

        let invalid_body = b"not a zip".to_vec();
        let (url, server) = serve_once(
            "200 OK",
            &format!("Content-Length: {}\r\n", invalid_body.len()),
            invalid_body,
        );
        assert!(
            try_download_chr_archive(&url, &partial, "chr-7.23.1.img", Duration::from_secs(2), false, None).is_err()
        );
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_download_is_retained_and_resumed() {
        let root = temporary_root("resume");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let archive_member = "chr-7.23.1.img";
        let body = chr_archive_body(archive_member);
        let split_at = body.len() / 2;
        let partial = root.join("download.part");

        let (url, server) = serve_once(
            "200 OK",
            &format!("Content-Length: {}\r\n", body.len()),
            body[..split_at].to_vec(),
        );
        assert!(try_download_chr_archive(&url, &partial, archive_member, Duration::from_secs(2), false, None).is_err());
        let first_request = server.join().unwrap();
        assert!(!first_request.contains("Range:"));
        assert_eq!(fs::read(&partial).unwrap(), body[..split_at]);

        let (url, server) = serve_once(
            "206 Partial Content",
            &format!(
                "Content-Length: {}\r\nContent-Range: bytes {split_at}-{}/{}\r\n",
                body.len() - split_at,
                body.len() - 1,
                body.len()
            ),
            body[split_at..].to_vec(),
        );
        try_download_chr_archive(&url, &partial, archive_member, Duration::from_secs(2), false, None).unwrap();
        let second_request = server.join().unwrap();
        assert!(second_request.contains(&format!("Range: bytes={split_at}-")));
        assert_eq!(fs::read(&partial).unwrap(), body);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn complete_response_replaces_partial_when_server_ignores_range() {
        let root = temporary_root("ignored-range");
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let archive_member = "chr-7.23.1.img";
        let body = chr_archive_body(archive_member);
        let split_at = body.len() / 2;
        let partial = root.join("download.part");
        fs::write(&partial, &body[..split_at]).unwrap();

        let (url, server) = serve_once("200 OK", &format!("Content-Length: {}\r\n", body.len()), body.clone());
        try_download_chr_archive(&url, &partial, archive_member, Duration::from_secs(2), false, None).unwrap();
        let request = server.join().unwrap();
        assert!(request.contains(&format!("Range: bytes={split_at}-")));
        assert_eq!(fs::read(&partial).unwrap(), body);

        fs::remove_dir_all(root).unwrap();
    }
}
