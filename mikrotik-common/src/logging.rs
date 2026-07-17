//! Tracing initialization helpers and logging macros.

#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

/// Initialize a `tracing-subscriber` from `RUST_LOG`, defaulting to `info`.
///
/// This is intended for examples and small CLIs that only need a stdout/stderr
/// formatter. Larger binaries should keep using their own logging setup when
/// they need file sinks, buffering, or additional layers.
#[cfg(feature = "tracing-subscriber")]
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Implement `Display` for a command enum with an inherent `as_path` method.
#[macro_export]
macro_rules! impl_command_display {
    ($type:ty) => {
        impl core::fmt::Display for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_path())
            }
        }
    };
}

/// Emit one `trace` event prefixed with a label.
#[macro_export]
macro_rules! trace_with_label {
    ($label:expr, $($argument:tt)*) => {{
        ::tracing::trace!("{}: {}", $label, format_args!($($argument)*))
    }};
}

/// Emit one `debug` event prefixed with a label.
#[macro_export]
macro_rules! debug_with_label {
    ($label:expr, $($argument:tt)*) => {{
        ::tracing::debug!("{}: {}", $label, format_args!($($argument)*))
    }};
}

/// Emit one `info` event prefixed with a label.
#[macro_export]
macro_rules! info_with_label {
    ($label:expr, $($argument:tt)*) => {{
        ::tracing::info!("{}: {}", $label, format_args!($($argument)*))
    }};
}

/// Emit one `warning` event prefixed with a label.
#[macro_export]
macro_rules! warn_with_label {
    ($label:expr, $($argument:tt)*) => {{
        ::tracing::warn!("{}: {}", $label, format_args!($($argument)*))
    }};
}

/// Emit one `error` event prefixed with a label.
#[macro_export]
macro_rules! error_with_label {
    ($label:expr, $($argument:tt)*) => {{
        ::tracing::error!("{}: {}", $label, format_args!($($argument)*))
    }};
}
