//! Formatting helpers.

use alloc::format;
use alloc::string::String;
use alloc::string::ToString as _;

/// Format a byte count using binary units.
#[must_use]
pub fn format_byte_count(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    let mut unit_index = 0;
    let mut unit = 1_u64;
    while bytes >= unit.saturating_mul(1024) && unit_index < UNITS.len() - 1 {
        unit = unit.saturating_mul(1024);
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes} B")
    } else {
        format_binary_unit(bytes, unit, UNITS[unit_index])
    }
}

/// Add underscores as thousands separators to an integer or decimal string.
///
/// Non-numeric input is returned unchanged.
#[must_use]
pub fn format_number_with_underscores(value: &str) -> String {
    let (mantissa, exponent) = value
        .find(['e', 'E'])
        .map_or((value, ""), |index| value.split_at(index));
    let (sign, unsigned) = mantissa.strip_prefix('-').map_or_else(
        || mantissa.strip_prefix('+').map_or(("", mantissa), |value| ("+", value)),
        |value| ("-", value),
    );
    let (integer, fraction) = unsigned
        .split_once('.')
        .map_or((unsigned, None), |(integer, fraction)| (integer, Some(fraction)));
    if integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|fraction| fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return value.to_string();
    }
    let separator_count = integer.len().saturating_sub(1) / 3;
    let mut output = String::with_capacity(value.len() + separator_count);
    output.push_str(sign);
    let first_group = integer.len() % 3;
    for (index, byte) in integer.bytes().enumerate() {
        if index > 0 && (index + 3 - first_group) % 3 == 0 {
            output.push('_');
        }
        output.push(char::from(byte));
    }
    if let Some(fraction) = fraction {
        output.push('.');
        output.push_str(fraction);
    }
    output.push_str(exponent);
    output
}

/// Format a numeric value as a percentage.
#[must_use]
pub fn format_percentage(value: &str) -> String {
    format_with_unit(value, "%", false)
}

/// Format a numeric value as degrees Celsius.
#[must_use]
pub fn format_temperature_celsius(value: &str) -> String {
    format_with_unit(value, "°C", true)
}

/// Format a numeric value as volts.
#[must_use]
pub fn format_voltage(value: &str) -> String {
    format_with_unit(value, "V", true)
}

/// Format a numeric value as amperes.
#[must_use]
pub fn format_current_amperes(value: &str) -> String {
    format_with_unit(value, "A", true)
}

/// Format a numeric value as watts.
#[must_use]
pub fn format_power_watts(value: &str) -> String {
    format_with_unit(value, "W", true)
}

/// Format a numeric frequency as megahertz.
#[must_use]
pub fn format_frequency_megahertz(value: &str) -> String {
    format_with_unit(value, "MHz", true)
}

/// Format a numeric rotational speed as revolutions per minute.
#[must_use]
pub fn format_rotational_speed_rpm(value: &str) -> String {
    format_with_unit(value, "RPM", true)
}

/// Normalize a numeric string and attach its unit.
fn format_with_unit(value: &str, unit: &str, separator: bool) -> String {
    let value = format_number_with_underscores(value);
    if separator {
        format!("{value} {unit}")
    } else {
        format!("{value}{unit}")
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
    use super::format_current_amperes;
    use super::format_frequency_megahertz;
    use super::format_number_with_underscores;
    use super::format_percentage;
    use super::format_power_watts;
    use super::format_rotational_speed_rpm;
    use super::format_temperature_celsius;
    use super::format_voltage;

    #[test]
    fn formats_binary_byte_counts() {
        assert_eq!(format_byte_count(42), "42 B");
        assert_eq!(format_byte_count(1536), "1.5 KiB");
        assert_eq!(format_byte_count(2 * 1024 * 1024), "2.0 MiB");
        assert_eq!(format_byte_count(3 * 1024 * 1024 * 1024 * 1024), "3.0 TiB");
    }

    #[test]
    fn formats_numeric_thousands_with_underscores() {
        assert_eq!(format_number_with_underscores("999"), "999");
        assert_eq!(format_number_with_underscores("1000"), "1_000");
        assert_eq!(format_number_with_underscores("-1234567.25"), "-1_234_567.25");
        assert_eq!(format_number_with_underscores("1.5e10"), "1.5e10");
        assert_eq!(format_number_with_underscores("not-a-number"), "not-a-number");
    }

    #[test]
    fn formats_measurements_with_units() {
        assert_eq!(format_percentage("37"), "37%");
        assert_eq!(format_temperature_celsius("42.5"), "42.5 °C");
        assert_eq!(format_voltage("24.1"), "24.1 V");
        assert_eq!(format_current_amperes("1.5"), "1.5 A");
        assert_eq!(format_power_watts("48"), "48 W");
        assert_eq!(format_frequency_megahertz("2000"), "2_000 MHz");
        assert_eq!(format_rotational_speed_rpm("12600"), "12_600 RPM");
    }
}
