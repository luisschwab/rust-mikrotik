//! System endpoint rows and `RouterOS` scalar formats.
//!
//! This module contains system inventory rows from `/system/*` plus reusable
//! scalar wrappers for `RouterOS` versions, dates, times, date-times, and
//! durations. The date parser accepts both current ISO-like API output and the
//! legacy month-name format seen on older `RouterOS` releases.

use alloc::string::String;
use alloc::string::ToString as _;
use core::fmt;
use core::ops::Range;
use core::str::FromStr;
use core::time::Duration;

use serde::Deserialize;
use serde::Serialize;
use time::Date;
use time::Month;
use time::PrimitiveDateTime;
use time::Time;

use crate::ParseError;
use crate::parse_non_empty;

/// `RouterOS` version string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RouterOsVersion(String);

impl RouterOsVersion {
    /// Return the raw version string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for RouterOsVersion {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_non_empty(value).map(Self)
    }
}

impl fmt::Display for RouterOsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RouterOsVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` local date/time without timezone information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsDateTime(PrimitiveDateTime);

impl RouterOsDateTime {
    /// Return the date/time value.
    #[must_use]
    pub const fn as_datetime(&self) -> PrimitiveDateTime {
        self.0
    }

    /// Return the date/time value by value.
    #[must_use]
    pub const fn into_datetime(self) -> PrimitiveDateTime {
        self.0
    }
}

impl FromStr for RouterOsDateTime {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_router_os_datetime(value).map(Self)
    }
}

impl fmt::Display for RouterOsDateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date = self.0.date();
        let time = self.0.time();

        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            date.year(),
            u8::from(date.month()),
            date.day(),
            time.hour(),
            time.minute(),
            time.second()
        )
    }
}

impl From<PrimitiveDateTime> for RouterOsDateTime {
    fn from(value: PrimitiveDateTime) -> Self {
        Self(value)
    }
}

impl From<RouterOsDateTime> for PrimitiveDateTime {
    fn from(value: RouterOsDateTime) -> Self {
        value.0
    }
}

impl Serialize for RouterOsDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` local date without timezone information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsDate(Date);

impl RouterOsDate {
    /// Return the date value.
    pub const fn as_date(&self) -> Date {
        self.0
    }
}

impl FromStr for RouterOsDate {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_router_os_date(value)
            .map(Self)
            .map_err(|_| ParseError::RouterOsDate)
    }
}

impl fmt::Display for RouterOsDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}",
            self.0.year(),
            u8::from(self.0.month()),
            self.0.day()
        )
    }
}

impl Serialize for RouterOsDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` local time without timezone information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsTime(Time);

impl RouterOsTime {
    /// Return the time value.
    pub const fn as_time(&self) -> Time {
        self.0
    }
}

impl FromStr for RouterOsTime {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_time_hms_as(value, ParseError::RouterOsTime).map(Self)
    }
}

impl fmt::Display for RouterOsTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.0.hour(), self.0.minute(), self.0.second())
    }
}

impl Serialize for RouterOsTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Parse either current ISO-like or legacy `RouterOS` date-time text.
fn parse_router_os_datetime(value: &str) -> Result<PrimitiveDateTime, ParseError> {
    if value.len() == 19 && value.as_bytes().get(4) == Some(&b'-') {
        parse_iso_like_datetime(value)
    } else {
        parse_legacy_datetime(value)
    }
}

/// Parse either current ISO-like or legacy `RouterOS` date text.
fn parse_router_os_date(value: &str) -> Result<Date, ParseError> {
    if value.len() == 10 && value.as_bytes().get(4) == Some(&b'-') {
        parse_iso_like_date_as(value, ParseError::RouterOsDate)
    } else {
        parse_legacy_date_as(value, ParseError::RouterOsDate)
    }
}

/// Parse `YYYY-MM-DD HH:MM:SS` date-time text.
fn parse_iso_like_datetime(value: &str) -> Result<PrimitiveDateTime, ParseError> {
    if value.as_bytes().get(10) != Some(&b' ') {
        return Err(ParseError::RouterOsDateTime);
    }
    let time = parse_time_hms(value.get(11..19).ok_or(ParseError::RouterOsDateTime)?)?;
    let date = parse_iso_like_date_as(&value[..10], ParseError::RouterOsDateTime)?;

    Ok(PrimitiveDateTime::new(date, time))
}

/// Parse `YYYY-MM-DD` date text using a caller-selected parse error.
fn parse_iso_like_date_as(value: &str, error: ParseError) -> Result<Date, ParseError> {
    let year = parse_i32_slice_as(value, 0..4, error)?;
    let month = parse_month_number_as(parse_u8_slice_as(value, 5..7, error)?, error)?;
    let day = parse_u8_slice_as(value, 8..10, error)?;

    if value.len() != 10 || value.as_bytes().get(4) != Some(&b'-') || value.as_bytes().get(7) != Some(&b'-') {
        return Err(error);
    }

    Date::from_calendar_date(year, month, day).map_err(|_| error)
}

/// Parse legacy `mon/DD/YYYY HH:MM:SS` date-time text.
fn parse_legacy_datetime(value: &str) -> Result<PrimitiveDateTime, ParseError> {
    let (date, time) = value.split_once(' ').ok_or(ParseError::RouterOsDateTime)?;
    Ok(PrimitiveDateTime::new(
        parse_legacy_date_as(date, ParseError::RouterOsDateTime)?,
        parse_time_hms(time)?,
    ))
}

/// Parse legacy `mon/DD/YYYY` date text using a caller-selected parse error.
fn parse_legacy_date_as(value: &str, error: ParseError) -> Result<Date, ParseError> {
    let mut date_parts = value.split('/');
    let month = parse_month_name_as(date_parts.next().ok_or(error)?, error)?;
    let day = parse_u8_as(date_parts.next().ok_or(error)?, error)?;
    let year = parse_i32_as(date_parts.next().ok_or(error)?, error)?;

    if date_parts.next().is_some() {
        return Err(error);
    }

    Date::from_calendar_date(year, month, day).map_err(|_| error)
}

/// Parse `HH:MM:SS` as a time using the date-time parse error.
fn parse_time_hms(value: &str) -> Result<Time, ParseError> {
    parse_time_hms_as(value, ParseError::RouterOsDateTime)
}

/// Parse `HH:MM:SS` as a time using a caller-selected parse error.
fn parse_time_hms_as(value: &str, error: ParseError) -> Result<Time, ParseError> {
    let mut parts = value.split(':');
    let hour = parse_u8_as(parts.next().ok_or(error)?, error)?;
    let minute = parse_u8_as(parts.next().ok_or(error)?, error)?;
    let second = parse_u8_as(parts.next().ok_or(error)?, error)?;

    if parts.next().is_some() {
        return Err(error);
    }

    Time::from_hms(hour, minute, second).map_err(|_| error)
}

/// Parse a three-letter English month name using a caller-selected parse error.
fn parse_month_name_as(value: &str, error: ParseError) -> Result<Month, ParseError> {
    if value.eq_ignore_ascii_case("jan") {
        Ok(Month::January)
    } else if value.eq_ignore_ascii_case("feb") {
        Ok(Month::February)
    } else if value.eq_ignore_ascii_case("mar") {
        Ok(Month::March)
    } else if value.eq_ignore_ascii_case("apr") {
        Ok(Month::April)
    } else if value.eq_ignore_ascii_case("may") {
        Ok(Month::May)
    } else if value.eq_ignore_ascii_case("jun") {
        Ok(Month::June)
    } else if value.eq_ignore_ascii_case("jul") {
        Ok(Month::July)
    } else if value.eq_ignore_ascii_case("aug") {
        Ok(Month::August)
    } else if value.eq_ignore_ascii_case("sep") {
        Ok(Month::September)
    } else if value.eq_ignore_ascii_case("oct") {
        Ok(Month::October)
    } else if value.eq_ignore_ascii_case("nov") {
        Ok(Month::November)
    } else if value.eq_ignore_ascii_case("dec") {
        Ok(Month::December)
    } else {
        Err(error)
    }
}

/// Parse a one-based month number using a caller-selected parse error.
fn parse_month_number_as(value: u8, error: ParseError) -> Result<Month, ParseError> {
    Month::try_from(value).map_err(|_| error)
}

/// Parse a byte range from a string as `i32` using a caller-selected parse error.
fn parse_i32_slice_as(value: &str, range: Range<usize>, error: ParseError) -> Result<i32, ParseError> {
    parse_i32_as(value.get(range).ok_or(error)?, error)
}

/// Parse a byte range from a string as `u8` using a caller-selected parse error.
fn parse_u8_slice_as(value: &str, range: Range<usize>, error: ParseError) -> Result<u8, ParseError> {
    parse_u8_as(value.get(range).ok_or(error)?, error)
}

/// Parse a string as `i32` using a caller-selected parse error.
fn parse_i32_as(value: &str, error: ParseError) -> Result<i32, ParseError> {
    value.parse().map_err(|_| error)
}

/// Parse a string as `u8` using a caller-selected parse error.
fn parse_u8_as(value: &str, error: ParseError) -> Result<u8, ParseError> {
    value.parse().map_err(|_| error)
}

/// A `RouterOS` duration (e.g. `4d17h7m22s`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RouterOsDuration {
    /// Finite duration value.
    Finite(Duration),
    /// `RouterOS` sentinel duration value `never`.
    Never,
}

impl RouterOsDuration {
    /// Return the [`RouterOsDuration`] as a [`Duration`].
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        match self {
            Self::Finite(duration) => *duration,
            Self::Never => Duration::MAX,
        }
    }
}

impl FromStr for RouterOsDuration {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "never" {
            Ok(Self::Never)
        } else {
            parse_router_os_duration(value).map(Self::Finite)
        }
    }
}

impl fmt::Display for RouterOsDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Finite(duration) => write_router_os_duration(*duration, f),
            Self::Never => f.write_str("never"),
        }
    }
}

impl From<Duration> for RouterOsDuration {
    fn from(value: Duration) -> Self {
        Self::Finite(value)
    }
}

impl From<RouterOsDuration> for Duration {
    fn from(value: RouterOsDuration) -> Self {
        value.as_duration()
    }
}

impl Serialize for RouterOsDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` duration range such as `0s..1m`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouterOsDurationRange {
    /// Inclusive range start.
    start: RouterOsDuration,
    /// Inclusive range end.
    end: RouterOsDuration,
}

impl RouterOsDurationRange {
    /// Return the start of the range.
    pub const fn start(&self) -> RouterOsDuration {
        self.start
    }

    /// Return the end of the range.
    pub const fn end(&self) -> RouterOsDuration {
        self.end
    }
}

impl FromStr for RouterOsDurationRange {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (start, end) = value.split_once("..").ok_or(ParseError::RouterOsDurationRange)?;

        Ok(Self {
            start: start.parse().map_err(|_| ParseError::RouterOsDurationRange)?,
            end: end.parse().map_err(|_| ParseError::RouterOsDurationRange)?,
        })
    }
}

impl fmt::Display for RouterOsDurationRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl Serialize for RouterOsDurationRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsDurationRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` byte count, accepting plain numbers and binary suffixes such as `16k`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsByteSize(u64);

impl RouterOsByteSize {
    /// Return the byte count.
    pub const fn bytes(self) -> u64 {
        self.0
    }
}

impl FromStr for RouterOsByteSize {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digit_count = value.bytes().take_while(u8::is_ascii_digit).count();
        if digit_count == 0 {
            return Err(ParseError::RouterOsByteSize);
        }

        let (number, suffix) = value.split_at(digit_count);
        let mut bytes = number.parse::<u64>().map_err(|_| ParseError::RouterOsByteSize)?;
        let multiplier = match suffix {
            "" => 1,
            "k" | "K" => 1024,
            "m" | "M" => 1024 * 1024,
            "g" | "G" => 1024 * 1024 * 1024,
            _ => return Err(ParseError::RouterOsByteSize),
        };
        bytes = bytes.checked_mul(multiplier).ok_or(ParseError::RouterOsByteSize)?;

        Ok(Self(bytes))
    }
}

impl fmt::Display for RouterOsByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for RouterOsByteSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsByteSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// `RouterOS` timezone offset in minutes east of UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsTimeZoneOffset(i16);

impl RouterOsTimeZoneOffset {
    /// Return offset minutes east of UTC.
    pub const fn minutes(self) -> i16 {
        self.0
    }
}

impl FromStr for RouterOsTimeZoneOffset {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let sign = match value.as_bytes().first() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return Err(ParseError::RouterOsTimeZoneOffset),
        };
        let (hours, minutes) = value
            .get(1..)
            .and_then(|value| value.split_once(':'))
            .ok_or(ParseError::RouterOsTimeZoneOffset)?;
        let hours = hours.parse::<i16>().map_err(|_| ParseError::RouterOsTimeZoneOffset)?;
        let minutes = minutes.parse::<i16>().map_err(|_| ParseError::RouterOsTimeZoneOffset)?;

        if hours > 23 || minutes > 59 {
            return Err(ParseError::RouterOsTimeZoneOffset);
        }

        Ok(Self(sign * (hours * 60 + minutes)))
    }
}

impl fmt::Display for RouterOsTimeZoneOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let absolute = self.0.unsigned_abs();

        write!(f, "{sign}{:02}:{:02}", absolute / 60, absolute % 60)
    }
}

impl Serialize for RouterOsTimeZoneOffset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RouterOsTimeZoneOffset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

/// Parse a `RouterOS` duration string into a standard duration.
fn parse_router_os_duration(value: &str) -> Result<Duration, ParseError> {
    if value.is_empty() {
        return Err(ParseError::RouterOsDuration);
    }

    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        return value
            .parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|_| ParseError::RouterOsDuration);
    }

    let mut remaining = value;
    let mut total = Duration::ZERO;

    while !remaining.is_empty() {
        let digit_count = remaining.bytes().take_while(u8::is_ascii_digit).count();

        if digit_count == 0 {
            return Err(ParseError::RouterOsDuration);
        }

        let (number, rest) = remaining.split_at(digit_count);
        let value = number.parse::<u64>().map_err(|_| ParseError::RouterOsDuration)?;
        let (unit, rest) = parse_duration_unit(rest)?;
        let component = unit.duration(value)?;

        total = total.checked_add(component).ok_or(ParseError::RouterOsDuration)?;
        remaining = rest;
    }

    Ok(total)
}

/// Units accepted by `RouterOS` duration strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DurationUnit {
    /// Week unit.
    Weeks,
    /// Day unit.
    Days,
    /// Hour unit.
    Hours,
    /// Minute unit.
    Minutes,
    /// Second unit.
    Seconds,
    /// Millisecond unit.
    Milliseconds,
    /// Microsecond unit.
    Microseconds,
    /// Nanosecond unit.
    Nanoseconds,
}

impl DurationUnit {
    /// Convert a numeric component in this unit into a duration.
    fn duration(self, value: u64) -> Result<Duration, ParseError> {
        match self {
            Self::Weeks => duration_from_secs(value, 7 * 24 * 60 * 60),
            Self::Days => duration_from_secs(value, 24 * 60 * 60),
            Self::Hours => duration_from_secs(value, 60 * 60),
            Self::Minutes => duration_from_secs(value, 60),
            Self::Seconds => Ok(Duration::from_secs(value)),
            Self::Milliseconds => Ok(Duration::from_millis(value)),
            Self::Microseconds => Ok(Duration::from_micros(value)),
            Self::Nanoseconds => Ok(Duration::from_nanos(value)),
        }
    }
}

/// Convert a seconds-based unit into a duration with overflow checking.
fn duration_from_secs(value: u64, multiplier: u64) -> Result<Duration, ParseError> {
    value
        .checked_mul(multiplier)
        .map(Duration::from_secs)
        .ok_or(ParseError::RouterOsDuration)
}

/// Parse the next duration unit suffix and return the remaining string.
fn parse_duration_unit(value: &str) -> Result<(DurationUnit, &str), ParseError> {
    if let Some(rest) = value.strip_prefix("min") {
        Ok((DurationUnit::Minutes, rest))
    } else if let Some(rest) = value.strip_prefix("ms") {
        Ok((DurationUnit::Milliseconds, rest))
    } else if let Some(rest) = value.strip_prefix("us") {
        Ok((DurationUnit::Microseconds, rest))
    } else if let Some(rest) = value.strip_prefix("ns") {
        Ok((DurationUnit::Nanoseconds, rest))
    } else if let Some(rest) = value.strip_prefix('w') {
        Ok((DurationUnit::Weeks, rest))
    } else if let Some(rest) = value.strip_prefix('d') {
        Ok((DurationUnit::Days, rest))
    } else if let Some(rest) = value.strip_prefix('h') {
        Ok((DurationUnit::Hours, rest))
    } else if let Some(rest) = value.strip_prefix('m') {
        Ok((DurationUnit::Minutes, rest))
    } else if let Some(rest) = value.strip_prefix('s') {
        Ok((DurationUnit::Seconds, rest))
    } else {
        Err(ParseError::RouterOsDuration)
    }
}

/// Format a standard duration using compact `RouterOS` duration units.
fn write_router_os_duration(duration: Duration, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut seconds = duration.as_secs();
    let milliseconds = duration.subsec_millis();
    let weeks = seconds / (7 * 24 * 60 * 60);
    seconds %= 7 * 24 * 60 * 60;
    let days = seconds / (24 * 60 * 60);
    seconds %= 24 * 60 * 60;
    let hours = seconds / (60 * 60);
    seconds %= 60 * 60;
    let minutes = seconds / 60;
    seconds %= 60;

    if weeks == 0 && days == 0 && hours == 0 && minutes == 0 && seconds == 0 && milliseconds == 0 {
        return f.write_str("0s");
    }

    if weeks > 0 {
        write!(f, "{weeks}w")?;
    }
    if days > 0 {
        write!(f, "{days}d")?;
    }
    if hours > 0 {
        write!(f, "{hours}h")?;
    }
    if minutes > 0 {
        write!(f, "{minutes}m")?;
    }
    if seconds > 0 {
        write!(f, "{seconds}s")?;
    }
    if milliseconds > 0 {
        write!(f, "{milliseconds}ms")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString;
    use core::time::Duration;

    use super::*;

    #[test]
    fn versions_are_non_empty_string_newtypes() {
        let version = "7.21.1 (stable)".parse::<RouterOsVersion>().unwrap();
        assert_eq!(version.as_str(), "7.21.1 (stable)");
        assert_eq!(version.to_string(), "7.21.1 (stable)");
        assert_eq!(serde_json::to_string(&version).unwrap(), r#""7.21.1 (stable)""#);
        assert_eq!(
            serde_json::from_str::<RouterOsVersion>(r#""7.22""#).unwrap().as_str(),
            "7.22"
        );
        assert!("".parse::<RouterOsVersion>().is_err());
    }

    #[test]
    fn date_times_accept_current_and_legacy_routeros_formats() {
        let current = "2026-09-06 17:30:45".parse::<RouterOsDateTime>().unwrap();
        let legacy = "sep/06/2026 17:30:45".parse::<RouterOsDateTime>().unwrap();
        assert_eq!(current, legacy);
        assert_eq!(current.to_string(), "2026-09-06 17:30:45");
        assert_eq!(RouterOsDateTime::from(current.as_datetime()), current);
        assert_eq!(PrimitiveDateTime::from(current), legacy.into_datetime());
        assert_eq!(serde_json::to_string(&legacy).unwrap(), r#""2026-09-06 17:30:45""#);
        assert_eq!(
            serde_json::from_str::<RouterOsDateTime>(r#""2026-09-06 17:30:45""#).unwrap(),
            legacy
        );
    }

    #[test]
    fn legacy_dates_accept_every_case_insensitive_month_name() {
        for (month, number) in [
            ("JAN", 1),
            ("feb", 2),
            ("mar", 3),
            ("apr", 4),
            ("may", 5),
            ("jun", 6),
            ("jul", 7),
            ("aug", 8),
            ("sep", 9),
            ("oct", 10),
            ("nov", 11),
            ("dec", 12),
        ] {
            let date = format!("{month}/01/2026").parse::<RouterOsDate>().unwrap();
            assert_eq!(date.to_string(), format!("2026-{number:02}-01"));
        }
    }

    #[test]
    fn dates_and_times_round_trip_through_accessors_and_json() {
        let date = "2024-02-29".parse::<RouterOsDate>().unwrap();
        assert_eq!(date.as_date().day(), 29);
        assert_eq!(serde_json::to_string(&date).unwrap(), r#""2024-02-29""#);
        assert_eq!(
            serde_json::from_str::<RouterOsDate>(r#""jan/02/2025""#)
                .unwrap()
                .to_string(),
            "2025-01-02"
        );

        let time = "01:02:03".parse::<RouterOsTime>().unwrap();
        assert_eq!(time.as_time().hour(), 1);
        assert_eq!(time.to_string(), "01:02:03");
        assert_eq!(serde_json::to_string(&time).unwrap(), r#""01:02:03""#);
        assert_eq!(
            serde_json::from_str::<RouterOsTime>(r#""23:59:59""#)
                .unwrap()
                .as_time()
                .second(),
            59
        );
    }

    #[test]
    fn date_and_time_parsers_reject_malformed_or_out_of_range_values() {
        for invalid in [
            "",
            "2026-13-01 00:00:00",
            "2026-01-32 00:00:00",
            "2026-01-01T00:00:00",
            "2026/01/01 00:00:00",
            "2026-01/01 00:00:00",
            "bad/01/2026 00:00:00",
            "jan/01/2026",
            "jan/01/2026 25:00:00",
            "jan/01/2026 00:00:00:00",
        ] {
            assert_eq!(invalid.parse::<RouterOsDateTime>(), Err(ParseError::RouterOsDateTime));
        }
        for invalid in [
            "2026-00-01",
            "2026-01-32",
            "2026/01/01",
            "2026-01/01",
            "bad/01/2026",
            "jan/01/2026/extra",
        ] {
            assert_eq!(invalid.parse::<RouterOsDate>(), Err(ParseError::RouterOsDate));
        }
        for invalid in ["", "24:00:00", "00:60:00", "00:00:60", "00:00:00:00"] {
            assert_eq!(invalid.parse::<RouterOsTime>(), Err(ParseError::RouterOsTime));
        }
    }

    #[test]
    fn durations_parse_all_units_and_convert_to_standard_duration() {
        let duration = "1w2d3h4m5s6ms7us8ns".parse::<RouterOsDuration>().unwrap();
        assert_eq!(duration.as_duration(), Duration::new(788_645, 6_007_008));
        assert_eq!(Duration::from(duration), duration.as_duration());
        assert_eq!(
            RouterOsDuration::from(Duration::from_secs(5)),
            RouterOsDuration::Finite(Duration::from_secs(5))
        );
        assert_eq!(
            "60".parse::<RouterOsDuration>().unwrap().as_duration(),
            Duration::from_secs(60)
        );
        assert_eq!(
            "1min".parse::<RouterOsDuration>().unwrap().as_duration(),
            Duration::from_secs(60)
        );
        assert_eq!(
            "never".parse::<RouterOsDuration>().unwrap().as_duration(),
            Duration::MAX
        );
    }

    #[test]
    fn durations_format_compactly_and_round_trip_through_json() {
        let duration = RouterOsDuration::from(Duration::from_millis(788_645_006));
        assert_eq!(duration.to_string(), "1w2d3h4m5s6ms");
        assert_eq!(RouterOsDuration::from(Duration::ZERO).to_string(), "0s");
        assert_eq!(RouterOsDuration::Never.to_string(), "never");
        assert_eq!(serde_json::to_string(&duration).unwrap(), r#""1w2d3h4m5s6ms""#);
        assert_eq!(
            serde_json::from_str::<RouterOsDuration>(r#""2h30m""#)
                .unwrap()
                .as_duration(),
            Duration::from_secs(9_000)
        );

        for invalid in ["", "ms", "1x", "1.5s", "18446744073709551615w"] {
            assert_eq!(invalid.parse::<RouterOsDuration>(), Err(ParseError::RouterOsDuration));
        }
    }

    #[test]
    fn duration_ranges_expose_bounds_and_validate_both_sides() {
        let range = "1s..2m".parse::<RouterOsDurationRange>().unwrap();
        assert_eq!(range.start().as_duration(), Duration::from_secs(1));
        assert_eq!(range.end().as_duration(), Duration::from_secs(120));
        assert_eq!(range.to_string(), "1s..2m");
        assert_eq!(serde_json::to_string(&range).unwrap(), r#""1s..2m""#);
        assert_eq!(
            serde_json::from_str::<RouterOsDurationRange>(r#""never..1h""#)
                .unwrap()
                .start(),
            RouterOsDuration::Never
        );
        for invalid in ["1s", "bad..1s", "1s..bad"] {
            assert_eq!(
                invalid.parse::<RouterOsDurationRange>(),
                Err(ParseError::RouterOsDurationRange)
            );
        }
    }

    #[test]
    fn byte_sizes_accept_binary_suffixes_and_detect_overflow() {
        for (wire, bytes) in [
            ("0", 0),
            ("16k", 16 * 1024),
            ("2M", 2 * 1024 * 1024),
            ("1g", 1024 * 1024 * 1024),
        ] {
            let size = wire.parse::<RouterOsByteSize>().unwrap();
            assert_eq!(size.bytes(), bytes);
            assert_eq!(size.to_string(), bytes.to_string());
        }
        assert_eq!(
            serde_json::to_string(&"1k".parse::<RouterOsByteSize>().unwrap()).unwrap(),
            r#""1024""#
        );
        assert_eq!(
            serde_json::from_str::<RouterOsByteSize>(r#""2048""#).unwrap().bytes(),
            2048
        );
        for invalid in ["", "k", "1t", "18446744073709551615k"] {
            assert_eq!(invalid.parse::<RouterOsByteSize>(), Err(ParseError::RouterOsByteSize));
        }
    }

    #[test]
    fn timezone_offsets_parse_format_and_validate_bounds() {
        let positive = "+05:30".parse::<RouterOsTimeZoneOffset>().unwrap();
        let negative = "-04:00".parse::<RouterOsTimeZoneOffset>().unwrap();
        assert_eq!(positive.minutes(), 330);
        assert_eq!(negative.minutes(), -240);
        assert_eq!(positive.to_string(), "+05:30");
        assert_eq!(negative.to_string(), "-04:00");
        assert_eq!(serde_json::to_string(&positive).unwrap(), r#""+05:30""#);
        assert_eq!(
            serde_json::from_str::<RouterOsTimeZoneOffset>(r#""-00:30""#)
                .unwrap()
                .minutes(),
            -30
        );
        for invalid in ["", "05:30", "+24:00", "+01:60", "+x:00", "+01:x", "+0100"] {
            assert_eq!(
                invalid.parse::<RouterOsTimeZoneOffset>(),
                Err(ParseError::RouterOsTimeZoneOffset)
            );
        }
    }
}
