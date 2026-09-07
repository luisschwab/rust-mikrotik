//! CHR image acquisition and download progress reporting.

mod download;
mod progress;

pub(crate) use self::download::IMAGES_DIR;
pub use self::download::MIKROTIK_ROUTEROS_DOWNLOAD_BASE_URL;
pub(crate) use self::download::chr_archive_filename;
pub(crate) use self::download::ensure_chr_image;
pub(crate) use self::progress::DownloadProgressGroup;
