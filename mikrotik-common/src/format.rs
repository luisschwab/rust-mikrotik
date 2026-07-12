//! Formatting helpers.

use alloc::format;
use alloc::string::String;

/// Format a byte count using binary units.
#[must_use]
pub fn format_byte_count(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    if bytes >= GIB {
        format_binary_unit(bytes, GIB, "GiB")
    } else if bytes >= MIB {
        format_binary_unit(bytes, MIB, "MiB")
    } else if bytes >= KIB {
        format_binary_unit(bytes, KIB, "KiB")
    } else {
        format!("{bytes} B")
    }
}

/// Format one binary unit with one rounded decimal place.
fn format_binary_unit(bytes: u64, unit: u64, suffix: &str) -> String {
    let scaled = ((u128::from(bytes) * 10) + (u128::from(unit) / 2)) / u128::from(unit);
    let whole = scaled / 10;
    let tenth = scaled % 10;
    format!("{whole}.{tenth} {suffix}")
}

#[cfg(test)]
mod tests {
    use super::format_byte_count;

    #[test]
    fn formats_binary_byte_counts() {
        assert_eq!(format_byte_count(42), "42 B");
        assert_eq!(format_byte_count(1536), "1.5 KiB");
        assert_eq!(format_byte_count(2 * 1024 * 1024), "2.0 MiB");
    }
}
