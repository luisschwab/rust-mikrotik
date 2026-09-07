//! CHR image cache, download, and archive extraction helpers.

use core::time::Duration;
use std::fs;
use std::io;
use std::io::IsTerminal as _;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use bitreq::Method;
use bitreq::Request;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use mikrotik_common::format::format_mebibytes;
use mikrotik_common::info_with_label;
use mikrotik_common::warn_with_label;
use terminal_size::Width;
use terminal_size::terminal_size_of;
use tracing::debug;

use crate::catalog::ChrArch;
use crate::error::Error;
use crate::error::Result;

/// Base URL for `MikroTik` `RouterOS` downloads.
pub const MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL: &str = "https://download.mikrotik.com/routeros/";

/// Directory for cached CHR base images.
pub(crate) const IMAGES_DIR: &str = ".chr-cache/images";

/// Return a cached CHR raw image [`PathBuf`], downloading and unpacking if needed.
pub(crate) fn ensure_chr_image(
    root: &Path,
    version: &str,
    arch: ChrArch,
    progress: Option<&MultiProgress>,
) -> Result<PathBuf> {
    let image = root.join(IMAGES_DIR).join(chr_image_filename(version, arch));
    if image.exists() {
        debug!("Using cached CHR {version} {arch:?} image {}", image.display());
        return Ok(image);
    }

    let archive_member = chr_image_filename(version, arch);
    let archive = root.join(IMAGES_DIR).join(chr_archive_filename(version, arch));
    let url = chr_url(version, arch);
    info_with_label!("CHR", "Downloading CHR {version} {arch:?} from {url}");
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
    progress: Option<&MultiProgress>,
) -> Result<()> {
    const ATTEMPTS: usize = 5;
    const TIMEOUT_SECONDS: u64 = 300;

    let partial = archive.with_extension("zip.part");
    let mut last_error = None;

    for attempt in 1..=ATTEMPTS {
        if partial.exists() {
            fs::remove_file(&partial)
                .map_err(|source| Error::io("remove stale partial CHR archive", &partial, source))?;
        }

        match try_download_chr_archive(url, &partial, archive_member, TIMEOUT_SECONDS, progress) {
            Ok(()) => {
                fs::rename(&partial, archive)
                    .map_err(|source| Error::io("promote partial CHR archive to", archive, source))?;
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
fn try_download_chr_archive(
    url: &str,
    partial: &Path,
    archive_member: &str,
    timeout_seconds: u64,
    progress_group: Option<&MultiProgress>,
) -> Result<()> {
    let response = Request::new(Method::Get, url)
        .with_timeout(timeout_seconds)
        .send_lazy()
        .map_err(|error| Error::Tool(format!("failed to GET {url}: {error}")))?;

    if !(200..300).contains(&response.status_code) {
        return Err(Error::Tool(format!(
            "request {url}: HTTP {} {}",
            response.status_code, response.reason_phrase
        )));
    }

    let expected_len = content_length(&response)?;
    let output =
        fs::File::create(partial).map_err(|source| Error::io("create partial CHR archive", partial, source))?;
    let archive_filename = format!("{archive_member}.zip");
    let mut progress = DownloadProgress::new(response, expected_len, &archive_filename, progress_group);
    let actual_len = io::copy(&mut progress, &mut io::BufWriter::new(output))
        .map_err(|source| Error::io("write partial CHR archive", partial, source))?;
    progress.finish();

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

/// A streaming CHR archive reader that reports download progress.
struct DownloadProgress<R> {
    /// Streaming HTTP response body.
    reader: R,
    /// Total archive size, when supplied by the HTTP server.
    expected_len: Option<u64>,
    /// Archive filename, abbreviated when needed to fit the terminal.
    filename: String,
    /// Width of the terminal progress bar in characters.
    bar_width: usize,
    /// Number of archive bytes written so far.
    downloaded: u64,
    /// Most recent terminal redraw time.
    last_render: Instant,
    /// Most recent non-interactive log line time.
    last_noninteractive_report: Instant,
    /// Whether stderr supports terminal redraws.
    interactive: bool,
    /// Multi-line-safe bar used for concurrent interactive downloads.
    progress_bar: Option<ProgressBar>,
}

impl<R> DownloadProgress<R> {
    /// Start counting a response body and, when appropriate, drawing its progress.
    fn new(reader: R, expected_len: Option<u64>, filename: &str, progress_group: Option<&MultiProgress>) -> Self {
        let (filename, bar_width) = progress_layout(filename, expected_len, terminal_columns());
        let terminal = io::stderr().is_terminal();
        let progress_bar = progress_group.filter(|_| terminal).map(|group| {
            let progress_bar = expected_len.map_or_else(ProgressBar::new_spinner, ProgressBar::new);
            let style = ProgressStyle::with_template("{msg}").unwrap_or_else(|_| ProgressStyle::default_spinner());
            progress_bar.set_style(style);
            group.add(progress_bar)
        });
        let progress = Self {
            reader,
            expected_len,
            filename,
            bar_width,
            downloaded: 0,
            last_render: Instant::now(),
            last_noninteractive_report: Instant::now(),
            interactive: terminal && progress_bar.is_none(),
            progress_bar,
        };
        if let Some(progress_bar) = &progress.progress_bar {
            progress_bar.set_message(progress.progress_line());
        }
        progress
    }

    /// Render the completed state in the appropriate output style.
    fn finish(&mut self) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.set_message(self.progress_line());
            progress_bar.finish();
        } else if self.interactive {
            self.render();
            eprintln!();
        } else if let Some(expected_len) = self.expected_len {
            eprintln!(
                "{} downloaded 100% {}/{}",
                self.filename,
                format_mebibytes(self.downloaded),
                format_mebibytes(expected_len)
            );
        } else {
            eprintln!("{} downloaded {}", self.filename, format_mebibytes(self.downloaded));
        }
    }

    /// Redraw the terminal progress bar using the latest byte count.
    fn render(&self) {
        eprint!("\r{}", self.progress_line());
    }

    /// Format the original compact progress-bar style.
    fn progress_line(&self) -> String {
        let Some(expected_len) = self.expected_len else {
            return format!("{} {}", self.filename, format_mebibytes(self.downloaded));
        };
        let percent = self.downloaded.saturating_mul(100) / expected_len.max(1);
        let filled = usize::try_from(percent.saturating_mul(self.bar_width as u64) / 100)
            .expect("percentage-derived progress width fits usize");
        let bar = format!("{}{}", "#".repeat(filled), "-".repeat(self.bar_width - filled));
        format!(
            "{} [{bar}] {percent:>3}% {}/{}",
            self.filename,
            format_mebibytes(self.downloaded),
            format_mebibytes(expected_len)
        )
    }

    /// Report the current state as one CI-friendly log line.
    fn render_noninteractive(&self) {
        if let Some(expected_len) = self.expected_len {
            let percent = self.downloaded.saturating_mul(100) / expected_len.max(1);
            eprintln!(
                "{} downloading {percent:>3}% {}/{}",
                self.filename,
                format_mebibytes(self.downloaded),
                format_mebibytes(expected_len)
            );
        } else {
            eprintln!("{} downloading {}", self.filename, format_mebibytes(self.downloaded));
        }
    }

    /// Return whether CI should receive another download heartbeat.
    fn should_report_noninteractive(&self) -> bool {
        self.last_noninteractive_report.elapsed() >= NONINTERACTIVE_PROGRESS_INTERVAL
    }
}

impl<R> io::Read for DownloadProgress<R>
where
    R: io::Read,
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read(buffer)?;
        self.downloaded +=
            u64::try_from(bytes_read).map_err(|_| io::Error::other("CHR download chunk length exceeds u64"))?;
        if bytes_read > 0 && self.progress_bar.is_some() && self.last_render.elapsed() >= Duration::from_millis(100) {
            if let Some(progress_bar) = &self.progress_bar {
                progress_bar.set_message(self.progress_line());
            }
            self.last_render = Instant::now();
        } else if bytes_read > 0 && self.interactive && self.last_render.elapsed() >= Duration::from_millis(100) {
            self.render();
            self.last_render = Instant::now();
        } else if bytes_read > 0 && !self.interactive && self.should_report_noninteractive() {
            self.render_noninteractive();
            self.last_noninteractive_report = Instant::now();
        }
        Ok(bytes_read)
    }
}

/// Interval between CI download heartbeat log lines.
const NONINTERACTIVE_PROGRESS_INTERVAL: Duration = Duration::from_secs(10);

/// Maximum number of characters in the terminal download progress bar.
const MAX_PROGRESS_BAR_WIDTH: usize = 40;
/// Minimum number of characters retained for the terminal download progress bar.
const MIN_PROGRESS_BAR_WIDTH: usize = 10;
/// Terminal width used when the operating system does not report one.
const DEFAULT_TERMINAL_COLUMNS: usize = 80;

/// Return the width of stderr's terminal, or a conservative conventional width.
fn terminal_columns() -> usize {
    terminal_size_of(io::stderr()).map_or(DEFAULT_TERMINAL_COLUMNS, |(Width(columns), _)| usize::from(columns))
}

/// Fit the archive label and progress bar into a terminal with the given number of columns.
fn progress_layout(filename: &str, expected_len: Option<u64>, columns: usize) -> (String, usize) {
    let counter_width = expected_len.map_or(20, |length| {
        let formatted = format_mebibytes(length);
        format!(" 100% {formatted}/{formatted}").chars().count()
    });
    // A preceding space plus the opening and closing brackets around the bar.
    let bar_overhead = 3;
    let filename_width = columns.saturating_sub(counter_width + bar_overhead + MIN_PROGRESS_BAR_WIDTH);
    let filename = abbreviate_filename(filename, filename_width);
    let bar_width = columns
        .saturating_sub(filename.chars().count() + counter_width + bar_overhead)
        .clamp(MIN_PROGRESS_BAR_WIDTH, MAX_PROGRESS_BAR_WIDTH);
    (filename, bar_width)
}

/// Abbreviate a filename with an ellipsis when it would not fit in the available width.
fn abbreviate_filename(filename: &str, width: usize) -> String {
    if filename.chars().count() <= width {
        return filename.to_owned();
    }
    if width <= 3 {
        return "...".chars().take(width).collect();
    }
    let prefix_width = width - 3;
    format!("{}...", filename.chars().take(prefix_width).collect::<String>())
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::process;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("mikrotik-chr-test-{}-{name}", process::id()))
    }

    fn serve_once(status: &str, headers: &str, body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let status = status.to_owned();
        let headers = headers.to_owned();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(stream, "HTTP/1.1 {status}\r\nConnection: close\r\n{headers}\r\n").unwrap();
            stream.write_all(&body).unwrap();
        });
        (format!("http://{address}/chr.zip"), server)
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
    fn progress_layout_does_not_wrap_an_eighty_column_terminal() {
        let filename = "chr-7.23.1-arm64.img.zip";
        let expected_len = 18 * 1024 * 1024 + 280 * 1024;
        let (display_filename, bar_width) = progress_layout(filename, Some(expected_len), 80);
        let formatted = format_mebibytes(expected_len);
        let line_width =
            display_filename.chars().count() + 3 + bar_width + format!(" 100% {formatted}/{formatted}").chars().count();

        assert_eq!(display_filename, filename);
        assert!(bar_width < MAX_PROGRESS_BAR_WIDTH);
        assert!(line_width <= 80);
    }

    #[test]
    fn noninteractive_progress_uses_a_ten_second_interval() {
        let mut progress = DownloadProgress::new(io::empty(), Some(100), "chr.img.zip", None);
        assert!(!progress.should_report_noninteractive());

        progress.last_noninteractive_report = Instant::now()
            .checked_sub(NONINTERACTIVE_PROGRESS_INTERVAL)
            .expect("current instant can represent a time ten seconds earlier");
        assert!(progress.should_report_noninteractive());
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
        let source = root.join("source.zip");
        let file = fs::File::create(&source).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file("chr-7.23.1.img", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"router image").unwrap();
        archive.finish().unwrap();
        let body = fs::read(&source).unwrap();

        let (url, server) = serve_once("200 OK", &format!("Content-Length: {}\r\n", body.len()), body.clone());
        let partial = root.join("download.part");
        try_download_chr_archive(&url, &partial, "chr-7.23.1.img", 2, None).unwrap();
        server.join().unwrap();
        assert_eq!(fs::read(&partial).unwrap(), body);

        let (url, server) = serve_once("404 Not Found", "Content-Length: 0\r\n", Vec::new());
        assert!(try_download_chr_archive(&url, &partial, "chr-7.23.1.img", 2, None).is_err());
        server.join().unwrap();

        let invalid_body = b"not a zip".to_vec();
        let (url, server) = serve_once(
            "200 OK",
            &format!("Content-Length: {}\r\n", invalid_body.len()),
            invalid_body,
        );
        assert!(try_download_chr_archive(&url, &partial, "chr-7.23.1.img", 2, None).is_err());
        server.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
