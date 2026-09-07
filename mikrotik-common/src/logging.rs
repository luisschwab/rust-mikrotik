//! Tracing initialization helpers and logging macros.

#[cfg(feature = "log")]
use tracing::Level;
#[cfg(feature = "log")]
use tracing_subscriber::EnvFilter;

/// Initialize a `tracing-subscriber` with an explicit filter.
///
/// This is intended for examples and small CLIs that only need a stdout/stderr
/// formatter. Larger binaries should keep using their own logging setup when
/// they need file sinks, buffering, or additional layers.
#[cfg(feature = "log")]
pub fn init_tracing(filter: Level) {
    let filter = EnvFilter::default().add_directive(filter.into());
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

#[cfg(all(test, feature = "log"))]
mod tests {
    use tracing::Level;

    use super::init_tracing;

    #[test]
    fn initializes_tracing_with_an_explicit_filter() {
        init_tracing(Level::ERROR);
    }
}
