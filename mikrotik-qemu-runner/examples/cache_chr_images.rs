//! Populate the local cache with every CHR image used by the version catalog.

use mikrotik_common::logging::init_tracing;
use mikrotik_qemu_runner::Result;
use tracing::Level;

fn main() -> Result<()> {
    init_tracing(Level::INFO);
    mikrotik_qemu_runner::cache_catalog_images()
}
