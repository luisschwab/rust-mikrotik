//! CHR download progress reporting for interactive terminals and CI logs.

use core::time::Duration;
use std::env;
use std::io;
use std::io::IsTerminal as _;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

use mikrotik_common::format::format_mebibytes;
use terminal_size::Height;
use terminal_size::Width;
use terminal_size::terminal_size_of;

/// Interval between CI download heartbeat log lines.
const NONINTERACTIVE_PROGRESS_INTERVAL: Duration = Duration::from_secs(10);

/// Maximum number of characters in the terminal download progress bar.
const MAX_PROGRESS_BAR_WIDTH: usize = 40;
/// Minimum number of characters retained for the terminal download progress bar.
const MIN_PROGRESS_BAR_WIDTH: usize = 10;
/// Minimum width reserved for each formatted MiB value.
const MIN_PROGRESS_SIZE_WIDTH: usize = 9;
/// Terminal width used when the operating system does not report one.
const DEFAULT_TERMINAL_COLUMNS: usize = 80;
/// Terminal height used when the operating system does not report one.
const DEFAULT_TERMINAL_ROWS: usize = 24;

/// Main-thread renderer for concurrent interactive downloads.
///
/// Workers only send state changes. The main thread owns every terminal write,
/// so a completed download and the remaining rows are redrawn as one frame.
pub(crate) struct DownloadProgressGroup {
    /// Cloneable event source passed to download workers.
    handle: DownloadProgressGroupHandle,
    /// Interactive renderer; absent for CI, redirected output, and dumb terminals.
    renderer: Option<DownloadProgressRenderer>,
}

impl DownloadProgressGroup {
    /// Create a progress group with a shared filename-column width.
    pub(crate) fn new(filename_width: usize) -> Self {
        let interactive = io::stderr().is_terminal() && env::var_os("TERM").is_none_or(|term| term != "dumb");
        let (sender, receiver) = mpsc::channel();
        Self {
            handle: DownloadProgressGroupHandle {
                interactive,
                next_id: Arc::new(AtomicUsize::new(0)),
                sender,
            },
            renderer: interactive.then(|| DownloadProgressRenderer::new(receiver, terminal_rows(), filename_width)),
        }
    }

    /// Return an event source for one or more download workers.
    pub(crate) fn handle(&self) -> DownloadProgressGroupHandle {
        self.handle.clone()
    }

    /// Apply pending worker updates and redraw the complete dynamic region.
    pub(crate) fn redraw(&mut self) -> io::Result<()> {
        if let Some(renderer) = &mut self.renderer {
            renderer.redraw(&mut io::stderr().lock(), terminal_columns())?;
        }
        Ok(())
    }

    /// Wait briefly for a worker update, waking immediately when one arrives.
    pub(crate) fn wait_for_update(&mut self, timeout: Duration) {
        if let Some(renderer) = &mut self.renderer {
            renderer.wait_for_update(timeout);
        } else {
            thread::sleep(timeout);
        }
    }
}

/// Cloneable worker side of a concurrent progress group.
#[derive(Clone)]
pub(crate) struct DownloadProgressGroupHandle {
    /// Whether the central renderer owns interactive output.
    interactive: bool,
    /// Monotonic entry identifier generator shared by workers.
    next_id: Arc<AtomicUsize>,
    /// State-change channel to the central renderer.
    sender: Sender<DownloadProgressEvent>,
}

impl DownloadProgressGroupHandle {
    /// Whether worker output must be routed through the central renderer.
    pub(super) fn is_interactive(&self) -> bool {
        self.interactive
    }

    /// Register a download with the central renderer.
    pub(super) fn add(&self, filename: &str) -> Option<DownloadProgressEntry> {
        if !self.interactive {
            return None;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.sender
            .send(DownloadProgressEvent::Register {
                id,
                filename: filename.to_owned(),
            })
            .ok()?;
        Some(DownloadProgressEntry {
            id,
            filename: filename.to_owned(),
            sender: self.sender.clone(),
            complete: false,
        })
    }
}

/// Worker-owned handle for one centrally rendered download row.
pub(super) struct DownloadProgressEntry {
    /// Renderer entry identifier.
    id: usize,
    /// Archive filename used in a terminal failure line.
    filename: String,
    /// State-change channel to the central renderer.
    sender: Sender<DownloadProgressEvent>,
    /// Whether the archive completed and passed validation.
    complete: bool,
}

impl DownloadProgressEntry {
    /// Create a non-owning event sink for the streaming reader.
    fn sink(&self) -> DownloadProgressSink {
        DownloadProgressSink {
            id: self.id,
            sender: self.sender.clone(),
        }
    }

    /// Queue a stable line above the live rows.
    pub(super) fn println(&self, message: String) {
        let _send_result = self.sender.send(DownloadProgressEvent::Println(message));
    }

    /// Mark this row complete after the archive has been validated and promoted.
    pub(super) fn complete(&mut self) {
        self.complete = true;
        let _send_result = self.sender.send(DownloadProgressEvent::Finish {
            id: self.id,
            failure: None,
        });
    }
}

impl Drop for DownloadProgressEntry {
    fn drop(&mut self) {
        if !self.complete {
            let _send_result = self.sender.send(DownloadProgressEvent::Finish {
                id: self.id,
                failure: Some(format!("{} download failed", self.filename)),
            });
        }
    }
}

/// Non-owning worker event sink without completion-on-drop behavior.
#[derive(Clone)]
struct DownloadProgressSink {
    /// Renderer entry identifier.
    id: usize,
    /// State-change channel to the central renderer.
    sender: Sender<DownloadProgressEvent>,
}

impl DownloadProgressSink {
    /// Publish current byte counts without writing to the terminal.
    fn update(&self, downloaded: u64, expected_len: Option<u64>) {
        let _send_result = self.sender.send(DownloadProgressEvent::Update {
            id: self.id,
            downloaded,
            expected_len,
        });
    }
}

/// One state transition sent from a download worker to the renderer.
enum DownloadProgressEvent {
    /// Add a newly started download in stable registration order.
    Register {
        /// Renderer entry identifier.
        id: usize,
        /// Full archive filename.
        filename: String,
    },
    /// Update a download's byte counts.
    Update {
        /// Renderer entry identifier.
        id: usize,
        /// Bytes received so far.
        downloaded: u64,
        /// Expected archive size, when supplied by the server.
        expected_len: Option<u64>,
    },
    /// Remove a finished download, optionally replacing its row with a failure.
    Finish {
        /// Renderer entry identifier.
        id: usize,
        /// Failure text replacing normal progress, when present.
        failure: Option<String>,
    },
    /// Print a durable message above the dynamic rows.
    Println(String),
}

/// Central state and cursor bookkeeping for concurrent progress rows.
struct DownloadProgressRenderer {
    /// Worker event receiver.
    receiver: Receiver<DownloadProgressEvent>,
    /// Downloads still represented by dynamic rows.
    entries: Vec<DownloadProgressRendererEntry>,
    /// Durable lines waiting to be printed above the dynamic rows.
    messages: Vec<String>,
    /// Number of dynamic rows emitted by the previous frame.
    previous_row_count: usize,
    /// Maximum dynamic rows that fit in the terminal.
    max_rows: usize,
    /// Precomputed width of the longest archive filename.
    filename_width: usize,
    /// Whether state changed since the previous frame.
    dirty: bool,
}

impl DownloadProgressRenderer {
    /// Create an empty renderer.
    fn new(receiver: Receiver<DownloadProgressEvent>, terminal_rows: usize, filename_width: usize) -> Self {
        Self {
            receiver,
            entries: Vec::new(),
            messages: Vec::new(),
            previous_row_count: 0,
            max_rows: terminal_rows.max(1),
            filename_width,
            dirty: false,
        }
    }

    /// Collect updates for one frame interval, waking early for durable output.
    fn wait_for_update(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return;
            };
            match self.receiver.recv_timeout(remaining) {
                Ok(event) => {
                    let urgent = matches!(
                        event,
                        DownloadProgressEvent::Finish { .. } | DownloadProgressEvent::Println(_)
                    );
                    self.apply(event);
                    if urgent {
                        return;
                    }
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Apply all queued changes, then emit one synchronized terminal frame.
    fn redraw(&mut self, writer: &mut impl io::Write, columns: usize) -> io::Result<()> {
        while let Ok(event) = self.receiver.try_recv() {
            self.apply(event);
        }
        if !self.dirty {
            return Ok(());
        }

        let size_width = self
            .entries
            .iter()
            .map(|entry| {
                let downloaded_width = format_mebibytes(entry.downloaded).chars().count();
                let expected_width = entry
                    .expected_len
                    .map_or(0, |expected_len| format_mebibytes(expected_len).chars().count());
                downloaded_width.max(expected_width)
            })
            .max()
            .unwrap_or(MIN_PROGRESS_SIZE_WIDTH)
            .max(MIN_PROGRESS_SIZE_WIDTH);
        let layout = progress_layout(self.filename_width, size_width, columns);

        let mut finished = Vec::new();
        self.entries.retain(|entry| {
            if entry.finished {
                finished.push(entry.render(layout, columns));
                false
            } else {
                true
            }
        });

        let mut durable_lines = core::mem::take(&mut self.messages);
        durable_lines.extend(finished);
        let pending_lines = self
            .entries
            .iter()
            .take(self.max_rows)
            .map(|entry| entry.render(layout, columns))
            .collect::<Vec<_>>();

        // A durable line consumes one row of the previous dynamic region. Any
        // rows left over after that must be overwritten or explicitly cleared.
        let stale_row_count = self.previous_row_count.saturating_sub(durable_lines.len());
        let rows_to_write = pending_lines.len().max(stale_row_count);

        write!(writer, "\x1b[?2026h")?;
        for line in durable_lines {
            write!(writer, "\r{line}\x1b[K\r\n")?;
        }
        for row in 0..rows_to_write {
            write!(writer, "\r")?;
            if let Some(line) = pending_lines.get(row) {
                write!(writer, "{line}")?;
            }
            write!(writer, "\x1b[K")?;
            if row + 1 < rows_to_write {
                write!(writer, "\r\n")?;
            }
        }

        if pending_lines.is_empty() {
            if rows_to_write > 0 {
                write!(writer, "\r\n")?;
            }
        } else if rows_to_write == 1 {
            write!(writer, "\x1b[0G")?;
        } else {
            write!(writer, "\x1b[{}F", rows_to_write - 1)?;
        }
        write!(writer, "\x1b[?2026l")?;
        writer.flush()?;

        self.previous_row_count = pending_lines.len();
        self.dirty = false;
        Ok(())
    }

    /// Apply one worker event without touching the terminal.
    fn apply(&mut self, event: DownloadProgressEvent) {
        match event {
            DownloadProgressEvent::Register { id, filename } => {
                self.entries.push(DownloadProgressRendererEntry {
                    id,
                    filename,
                    downloaded: 0,
                    expected_len: None,
                    finished: false,
                    failure: None,
                });
            }
            DownloadProgressEvent::Update {
                id,
                downloaded,
                expected_len,
            } => {
                if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
                    entry.downloaded = downloaded;
                    entry.expected_len = expected_len;
                }
            }
            DownloadProgressEvent::Finish { id, failure } => {
                if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
                    entry.finished = true;
                    entry.failure = failure;
                }
            }
            DownloadProgressEvent::Println(message) => self.messages.push(message),
        }
        self.dirty = true;
    }
}

/// Download state formatted by the central renderer at the current terminal width.
struct DownloadProgressRendererEntry {
    /// Registration identifier.
    id: usize,
    /// Full archive filename.
    filename: String,
    /// Bytes received so far.
    downloaded: u64,
    /// Expected archive size, when supplied by the server.
    expected_len: Option<u64>,
    /// Whether this entry should become a durable line on the next frame.
    finished: bool,
    /// Failure text replacing normal progress, when present.
    failure: Option<String>,
}

impl DownloadProgressRendererEntry {
    /// Render this entry without writing to the terminal.
    fn render(&self, layout: ProgressLineLayout, columns: usize) -> String {
        self.failure.as_ref().map_or_else(
            || progress_line_with_layout(&self.filename, self.expected_len, self.downloaded, layout),
            |failure| abbreviate_filename(failure, columns.saturating_sub(1)),
        )
    }
}

/// A streaming CHR archive reader that reports download progress.
pub(super) struct DownloadProgress<R> {
    /// Streaming HTTP response body.
    reader: R,
    /// Total archive size, when supplied by the HTTP server.
    expected_len: Option<u64>,
    /// Archive filename.
    filename: String,
    /// Column widths used for single-download terminal output.
    layout: ProgressLineLayout,
    /// Number of archive bytes written so far.
    downloaded: u64,
    /// Most recent terminal redraw time.
    last_render: Instant,
    /// Most recent non-interactive log line time.
    last_noninteractive_report: Instant,
    /// Whether stderr supports terminal redraws.
    interactive: bool,
    /// Worker-to-renderer event sink for a concurrent interactive download.
    progress_sink: Option<DownloadProgressSink>,
}

impl<R> DownloadProgress<R> {
    /// Start counting a response body and, when appropriate, drawing its progress.
    pub(super) fn new(
        reader: R,
        expected_len: Option<u64>,
        downloaded: u64,
        filename: &str,
        grouped_output: bool,
        progress_entry: Option<&DownloadProgressEntry>,
    ) -> Self {
        let size_width = expected_len.map_or(MIN_PROGRESS_SIZE_WIDTH, |expected_len| {
            format_mebibytes(expected_len)
                .chars()
                .count()
                .max(MIN_PROGRESS_SIZE_WIDTH)
        });
        let layout = progress_layout(filename.chars().count(), size_width, terminal_columns());
        let terminal = io::stderr().is_terminal();
        let progress = Self {
            reader,
            expected_len,
            filename: filename.to_owned(),
            layout,
            downloaded,
            last_render: Instant::now(),
            last_noninteractive_report: Instant::now(),
            interactive: terminal && !grouped_output,
            progress_sink: progress_entry.map(DownloadProgressEntry::sink),
        };
        if let Some(progress_sink) = &progress.progress_sink {
            progress_sink.update(progress.downloaded, progress.expected_len);
        }
        progress
    }

    /// Render the completed state in the appropriate output style.
    pub(super) fn finish(&mut self) {
        if let Some(progress_sink) = &self.progress_sink {
            progress_sink.update(self.downloaded, self.expected_len);
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
        progress_line_with_layout(&self.filename, self.expected_len, self.downloaded, self.layout)
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
        if bytes_read > 0 {
            match &self.progress_sink {
                Some(progress_sink) => {
                    if self.last_render.elapsed() >= Duration::from_millis(100) {
                        progress_sink.update(self.downloaded, self.expected_len);
                        self.last_render = Instant::now();
                    }
                }
                None if self.interactive => {
                    if self.last_render.elapsed() >= Duration::from_millis(100) {
                        self.render();
                        self.last_render = Instant::now();
                    }
                }
                None => {
                    if self.should_report_noninteractive() {
                        self.render_noninteractive();
                        self.last_noninteractive_report = Instant::now();
                    }
                }
            }
        }
        Ok(bytes_read)
    }
}

/// Return the width of stderr's terminal, or a conservative conventional width.
fn terminal_columns() -> usize {
    terminal_size_of(io::stderr()).map_or(DEFAULT_TERMINAL_COLUMNS, |(Width(columns), _)| usize::from(columns))
}

/// Return the height of stderr's terminal, or a conservative conventional height.
fn terminal_rows() -> usize {
    terminal_size_of(io::stderr()).map_or(DEFAULT_TERMINAL_ROWS, |(_, Height(rows))| usize::from(rows))
}

/// Shared column widths for one or more progress rows.
#[derive(Clone, Copy)]
struct ProgressLineLayout {
    /// Width of the padded or abbreviated filename column.
    filename_width: usize,
    /// Width of the progress bar body.
    bar_width: usize,
    /// Width of each right-aligned file-size column.
    size_width: usize,
}

/// Format one progress line using shared column widths.
fn progress_line_with_layout(
    filename: &str,
    expected_len: Option<u64>,
    downloaded: u64,
    layout: ProgressLineLayout,
) -> String {
    let filename = abbreviate_filename(filename, layout.filename_width);
    let (percent, filled, expected_size) = if let Some(expected_len) = expected_len {
        let percent = (downloaded.saturating_mul(100) / expected_len.max(1)).min(100);
        let filled = usize::try_from(percent.saturating_mul(layout.bar_width as u64) / 100)
            .expect("percentage-derived progress width fits usize");
        (percent.to_string(), filled, format_mebibytes(expected_len))
    } else {
        ("--".to_owned(), 0, "--".to_owned())
    };
    let bar = format!("{}{}", "#".repeat(filled), "-".repeat(layout.bar_width - filled));
    let downloaded_size = format_mebibytes(downloaded);
    let filename_width = layout.filename_width;
    let size_width = layout.size_width;
    format!(
        "{filename:<filename_width$} [{bar}] {percent:>3}% {downloaded_size:>size_width$}/{expected_size:>size_width$}"
    )
}

/// Fit shared filename, progress-bar, and file-size columns into a terminal.
fn progress_layout(filename_width: usize, size_width: usize, columns: usize) -> ProgressLineLayout {
    // Leave the final terminal column unused: writing into it can implicitly
    // wrap and desynchronize a multi-line cursor-based renderer.
    let columns = columns.saturating_sub(1);
    // Leading space, three percentage characters, percent sign, separating
    // space, two size columns, and the slash between them.
    let counter_width = 7 + size_width.saturating_mul(2);
    // A preceding space plus the opening and closing brackets around the bar.
    let bar_overhead = 3;
    let fixed_width = counter_width + bar_overhead;
    let filename_width = filename_width.min(columns.saturating_sub(fixed_width + MIN_PROGRESS_BAR_WIDTH));
    let bar_width = columns
        .saturating_sub(filename_width + fixed_width)
        .min(MAX_PROGRESS_BAR_WIDTH);
    ProgressLineLayout {
        filename_width,
        bar_width,
        size_width,
    }
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
    use std::io;
    use std::io::Read as _;
    use std::time::Instant;

    use mikrotik_common::format::format_mebibytes;

    use super::*;

    #[test]
    fn progress_layout_does_not_wrap_an_eighty_column_terminal() {
        let filename = "chr-7.23.1-arm64.img.zip";
        let expected_len = 18 * 1024 * 1024 + 280 * 1024;
        let size_width = format_mebibytes(expected_len)
            .chars()
            .count()
            .max(MIN_PROGRESS_SIZE_WIDTH);
        let layout = progress_layout(filename.chars().count(), size_width, 80);
        let line = progress_line_with_layout(filename, Some(expected_len), expected_len, layout);

        assert_eq!(layout.filename_width, filename.chars().count());
        assert!(layout.bar_width < MAX_PROGRESS_BAR_WIDTH);
        assert!(line.chars().count() < 80);
    }

    #[test]
    fn shared_layout_aligns_filenames_bars_percentages_and_sizes() {
        let short_filename = "chr-7.24-arm64.img.zip";
        let long_filename = "chr-7.23.1-arm64.img.zip";
        let short_expected = 18 * 1024 * 1024;
        let long_expected = 123 * 1024 * 1024;
        let filename_width = long_filename.chars().count();
        let size_width = format_mebibytes(long_expected).chars().count();
        let layout = progress_layout(filename_width, size_width, 120);

        let short_line = progress_line_with_layout(short_filename, Some(short_expected), 5 * 1024 * 1024, layout);
        let long_line = progress_line_with_layout(long_filename, Some(long_expected), 105 * 1024 * 1024, layout);
        let section_columns = |line: &str| {
            (
                line.find('[').unwrap(),
                line.find(']').unwrap(),
                line.find('%').unwrap(),
                line.find('/').unwrap(),
            )
        };

        assert_eq!(section_columns(&short_line), section_columns(&long_line));
        assert_eq!(short_line.chars().count(), long_line.chars().count());
    }

    #[test]
    fn noninteractive_progress_uses_a_ten_second_interval() {
        let mut progress = DownloadProgress::new(io::empty(), Some(100), 0, "chr.img.zip", false, None);
        assert!(!progress.should_report_noninteractive());

        progress.last_noninteractive_report = Instant::now()
            .checked_sub(NONINTERACTIVE_PROGRESS_INTERVAL)
            .expect("current instant can represent a time ten seconds earlier");
        assert!(progress.should_report_noninteractive());
    }

    #[test]
    fn grouped_progress_uses_explicit_row_boundaries_when_one_download_finishes() {
        let (sender, receiver) = mpsc::channel();
        let filename_width = "chr-7.23.1-arm64.img.zip".chars().count();
        let mut renderer = DownloadProgressRenderer::new(receiver, 8, filename_width);
        let layout = progress_layout(filename_width, MIN_PROGRESS_SIZE_WIDTH, 100);
        sender
            .send(DownloadProgressEvent::Register {
                id: 0,
                filename: "chr-7.23.1-arm64.img.zip".to_owned(),
            })
            .unwrap();
        sender
            .send(DownloadProgressEvent::Update {
                id: 0,
                downloaded: 5 * 1024 * 1024,
                expected_len: Some(18 * 1024 * 1024),
            })
            .unwrap();
        sender
            .send(DownloadProgressEvent::Register {
                id: 1,
                filename: "chr-7.23.2-arm64.img.zip".to_owned(),
            })
            .unwrap();
        sender
            .send(DownloadProgressEvent::Update {
                id: 1,
                downloaded: 3 * 1024 * 1024,
                expected_len: Some(18 * 1024 * 1024),
            })
            .unwrap();

        let mut initial_frame = Vec::new();
        renderer.redraw(&mut initial_frame, 100).unwrap();
        let first_initial = progress_line_with_layout(
            "chr-7.23.1-arm64.img.zip",
            Some(18 * 1024 * 1024),
            5 * 1024 * 1024,
            layout,
        );
        let second_initial = progress_line_with_layout(
            "chr-7.23.2-arm64.img.zip",
            Some(18 * 1024 * 1024),
            3 * 1024 * 1024,
            layout,
        );
        let initial_frame = String::from_utf8(initial_frame).unwrap();
        assert!(initial_frame.contains(&format!("\r{first_initial}\x1b[K\r\n\r{second_initial}\x1b[K")));

        sender
            .send(DownloadProgressEvent::Update {
                id: 0,
                downloaded: 18 * 1024 * 1024,
                expected_len: Some(18 * 1024 * 1024),
            })
            .unwrap();
        sender
            .send(DownloadProgressEvent::Finish { id: 0, failure: None })
            .unwrap();
        sender
            .send(DownloadProgressEvent::Update {
                id: 1,
                downloaded: 7 * 1024 * 1024,
                expected_len: Some(18 * 1024 * 1024),
            })
            .unwrap();

        let mut completion_frame = Vec::new();
        renderer.redraw(&mut completion_frame, 100).unwrap();
        let completed = progress_line_with_layout(
            "chr-7.23.1-arm64.img.zip",
            Some(18 * 1024 * 1024),
            18 * 1024 * 1024,
            layout,
        );
        let remaining = progress_line_with_layout(
            "chr-7.23.2-arm64.img.zip",
            Some(18 * 1024 * 1024),
            7 * 1024 * 1024,
            layout,
        );
        let completion_frame = String::from_utf8(completion_frame).unwrap();
        assert!(completion_frame.contains(&format!("\r{completed}\x1b[K\r\n\r{remaining}\x1b[K")));
        assert!(!completion_frame.contains(&format!("{completed}{remaining}")));
    }

    #[test]
    fn grouped_progress_never_falls_through_to_noninteractive_reporting() {
        let (sender, _receiver) = mpsc::channel();
        let entry = DownloadProgressEntry {
            id: 0,
            filename: "chr-7.23.1-arm64.img.zip".to_owned(),
            sender,
            complete: false,
        };
        let mut progress = DownloadProgress::new(
            io::Cursor::new([0_u8; 16]),
            Some(16),
            0,
            "chr-7.23.1-arm64.img.zip",
            true,
            Some(&entry),
        );
        let heartbeat = Instant::now()
            .checked_sub(NONINTERACTIVE_PROGRESS_INTERVAL)
            .expect("current instant can represent a time ten seconds earlier");
        progress.last_noninteractive_report = heartbeat;
        progress.last_render = Instant::now();

        let mut byte = [0];
        progress.read_exact(&mut byte).unwrap();

        assert_eq!(progress.last_noninteractive_report, heartbeat);
    }
}
