//! System endpoint rows and `RouterOS` scalar formats.
//!
//! This module contains system inventory rows from `/system/*` plus reusable
//! scalar wrappers for `RouterOS` versions, dates, times, date-times, and
//! durations. The date parser accepts both current ISO-like API output and the
//! legacy month-name format seen on older `RouterOS` releases.

use alloc::string::String;
use alloc::string::ToString as _;
use core::fmt;
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date = self.0.date();
        let time = self.0.time();

        write!(
            formatter,
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
        let year = parse_i32_slice_as(value, 0..4, ParseError::RouterOsDate)?;
        let month = parse_month_number(parse_u8_slice_as(value, 5..7, ParseError::RouterOsDate)?)?;
        let day = parse_u8_slice_as(value, 8..10, ParseError::RouterOsDate)?;

        if value.len() != 10 || value.as_bytes().get(4) != Some(&b'-') || value.as_bytes().get(7) != Some(&b'-') {
            return Err(ParseError::RouterOsDate);
        }

        Date::from_calendar_date(year, month, day)
            .map(Self)
            .map_err(|_| ParseError::RouterOsDate)
    }
}

impl fmt::Display for RouterOsDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:02}:{:02}:{:02}",
            self.0.hour(),
            self.0.minute(),
            self.0.second()
        )
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

/// Parse `YYYY-MM-DD HH:MM:SS` date-time text.
fn parse_iso_like_datetime(value: &str) -> Result<PrimitiveDateTime, ParseError> {
    let year = parse_i32_slice(value, 0..4)?;
    let month = parse_month_number(parse_u8_slice(value, 5..7)?)?;
    let day = parse_u8_slice(value, 8..10)?;
    let time = parse_time_hms(value.get(11..19).ok_or(ParseError::RouterOsDateTime)?)?;
    let date = Date::from_calendar_date(year, month, day).map_err(|_| ParseError::RouterOsDateTime)?;

    Ok(PrimitiveDateTime::new(date, time))
}

/// Parse legacy `mon/DD/YYYY HH:MM:SS` date-time text.
fn parse_legacy_datetime(value: &str) -> Result<PrimitiveDateTime, ParseError> {
    let (date, time) = value.split_once(' ').ok_or(ParseError::RouterOsDateTime)?;
    let mut date_parts = date.split('/');
    let month = parse_month_name(date_parts.next().ok_or(ParseError::RouterOsDateTime)?)?;
    let day = parse_u8(date_parts.next().ok_or(ParseError::RouterOsDateTime)?)?;
    let year = parse_i32(date_parts.next().ok_or(ParseError::RouterOsDateTime)?)?;

    if date_parts.next().is_some() {
        return Err(ParseError::RouterOsDateTime);
    }

    let date = Date::from_calendar_date(year, month, day).map_err(|_| ParseError::RouterOsDateTime)?;
    Ok(PrimitiveDateTime::new(date, parse_time_hms(time)?))
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

/// Parse a three-letter English month name used by legacy `RouterOS`.
fn parse_month_name(value: &str) -> Result<Month, ParseError> {
    match value {
        "Jan" => Ok(Month::January),
        "Feb" => Ok(Month::February),
        "Mar" => Ok(Month::March),
        "Apr" => Ok(Month::April),
        "May" => Ok(Month::May),
        "Jun" => Ok(Month::June),
        "Jul" => Ok(Month::July),
        "Aug" => Ok(Month::August),
        "Sep" => Ok(Month::September),
        "Oct" => Ok(Month::October),
        "Nov" => Ok(Month::November),
        "Dec" => Ok(Month::December),
        _ => Err(ParseError::RouterOsDateTime),
    }
}

/// Parse a one-based month number.
fn parse_month_number(value: u8) -> Result<Month, ParseError> {
    Month::try_from(value).map_err(|_| ParseError::RouterOsDateTime)
}

/// Parse a byte range from a string as `i32` using the date-time parse error.
fn parse_i32_slice(value: &str, range: core::ops::Range<usize>) -> Result<i32, ParseError> {
    parse_i32_slice_as(value, range, ParseError::RouterOsDateTime)
}

/// Parse a byte range from a string as `u8` using the date-time parse error.
fn parse_u8_slice(value: &str, range: core::ops::Range<usize>) -> Result<u8, ParseError> {
    parse_u8_slice_as(value, range, ParseError::RouterOsDateTime)
}

/// Parse a byte range from a string as `i32` using a caller-selected parse error.
fn parse_i32_slice_as(value: &str, range: core::ops::Range<usize>, error: ParseError) -> Result<i32, ParseError> {
    parse_i32_as(value.get(range).ok_or(error)?, error)
}

/// Parse a byte range from a string as `u8` using a caller-selected parse error.
fn parse_u8_slice_as(value: &str, range: core::ops::Range<usize>, error: ParseError) -> Result<u8, ParseError> {
    parse_u8_as(value.get(range).ok_or(error)?, error)
}

/// Parse a string as `i32` using the date-time parse error.
fn parse_i32(value: &str) -> Result<i32, ParseError> {
    parse_i32_as(value, ParseError::RouterOsDateTime)
}

/// Parse a string as `u8` using the date-time parse error.
fn parse_u8(value: &str) -> Result<u8, ParseError> {
    parse_u8_as(value, ParseError::RouterOsDateTime)
}

/// Parse a string as `i32` using a caller-selected parse error.
fn parse_i32_as(value: &str, error: ParseError) -> Result<i32, ParseError> {
    value.parse().map_err(|_| error)
}

/// Parse a string as `u8` using a caller-selected parse error.
fn parse_u8_as(value: &str, error: ParseError) -> Result<u8, ParseError> {
    value.parse().map_err(|_| error)
}

/// `RouterOS` duration such as `4d17h7m22s`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RouterOsDuration(Duration);

impl RouterOsDuration {
    /// Return the duration value.
    #[must_use]
    pub const fn as_duration(&self) -> Duration {
        self.0
    }

    /// Return the duration value by value.
    #[must_use]
    pub const fn into_duration(self) -> Duration {
        self.0
    }
}

impl FromStr for RouterOsDuration {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_router_os_duration(value).map(Self)
    }
}

impl fmt::Display for RouterOsDuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_router_os_duration(self.0, formatter)
    }
}

impl From<Duration> for RouterOsDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<RouterOsDuration> for Duration {
    fn from(value: RouterOsDuration) -> Self {
        value.0
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}..{}", self.start, self.end)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { '-' } else { '+' };
        let absolute = self.0.unsigned_abs();

        write!(formatter, "{sign}{:02}:{:02}", absolute / 60, absolute % 60)
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
fn write_router_os_duration(duration: Duration, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        return formatter.write_str("0s");
    }

    if weeks > 0 {
        write!(formatter, "{weeks}w")?;
    }
    if days > 0 {
        write!(formatter, "{days}d")?;
    }
    if hours > 0 {
        write!(formatter, "{hours}h")?;
    }
    if minutes > 0 {
        write!(formatter, "{minutes}m")?;
    }
    if seconds > 0 {
        write!(formatter, "{seconds}s")?;
    }
    if milliseconds > 0 {
        write!(formatter, "{milliseconds}ms")?;
    }

    Ok(())
}
