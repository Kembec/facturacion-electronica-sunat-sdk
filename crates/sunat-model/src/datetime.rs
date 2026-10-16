use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use crate::ModelError;

/// Peru has kept UTC-05:00 without daylight saving since 1994; documents carry their dates as
/// `America/Lima` wall-clock time, so a fixed offset gives the right one for any modern document.
const LIMA_OFFSET_SECONDS: i64 = -5 * 3600;

/// An instant given as `YYYY-MM-DDTHH:MM:SS±HH:MM` (or `Z`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    unix_seconds: i64,
}

/// A calendar date given as `YYYY-MM-DD`, without time or offset: the dates a daily summary and a
/// void communication carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    days: i64,
}

/// A wall-clock date and time in Lima.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimaDateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl DateTime {
    pub fn unix_seconds(&self) -> i64 {
        self.unix_seconds
    }

    pub fn to_lima(&self) -> LimaDateTime {
        let local = self.unix_seconds + LIMA_OFFSET_SECONDS;
        let days = local.div_euclid(86_400);
        let secs = local.rem_euclid(86_400) as u32;
        let (year, month, day) = civil_from_days(days);
        LimaDateTime { year, month, day, hour: secs / 3600, minute: secs / 60 % 60, second: secs % 60 }
    }
}

impl LimaDateTime {
    /// `YYYY-MM-DD`.
    pub fn date_string(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// `HH:MM:SS`.
    pub fn time_string(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

impl Date {
    /// `YYYY-MM-DD`.
    pub fn date_string(&self) -> String {
        let (year, month, day) = civil_from_days(self.days);
        format!("{year:04}-{month:02}-{day:02}")
    }

    /// `YYYYMMDD`, the form in the identifiers `RC-YYYYMMDD-N` and `RA-YYYYMMDD-N`.
    pub fn compact(&self) -> String {
        let (year, month, day) = civil_from_days(self.days);
        format!("{year:04}{month:02}{day:02}")
    }
}

impl FromStr for Date {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_date(text.as_bytes())
            .map(|days| Date { days })
            .ok_or_else(|| ModelError::invalid("$", format!("not a date (YYYY-MM-DD): {text:?}")))
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(|e: ModelError| serde::de::Error::custom(e.to_string()))
    }
}

impl FromStr for DateTime {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_atom(text).ok_or_else(|| {
            ModelError::invalid("$", format!("not ISO 8601 with offset (YYYY-MM-DDTHH:MM:SS±HH:MM): {text:?}"))
        })
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(|e: ModelError| serde::de::Error::custom(e.to_string()))
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lima = self.to_lima();
        write!(f, "{}T{}-05:00", lima.date_string(), lima.time_string())
    }
}

fn parse_atom(text: &str) -> Option<DateTime> {
    let b = text.as_bytes();
    if b.len() < 20 || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let days = parse_date(&b[0..10])?;
    let hour = number(&b[11..13])?;
    let minute = number(&b[14..16])?;
    let second = number(&b[17..19])?;
    let offset = parse_offset(&b[19..])?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let local = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(DateTime { unix_seconds: local - offset })
}

/// `YYYY-MM-DD` (exactly 10 bytes) to days since 1970-01-01.
fn parse_date(b: &[u8]) -> Option<i64> {
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let year = number(&b[0..4])?;
    let month = number(&b[5..7])?;
    let day = number(&b[8..10])?;
    let valid_date = (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month);
    if year < 1 || !valid_date {
        return None;
    }
    Some(days_from_civil(year, month as u32, day as u32))
}

fn parse_offset(b: &[u8]) -> Option<i64> {
    if b == b"Z" {
        return Some(0);
    }
    if b.len() != 6 || b[3] != b':' {
        return None;
    }
    let sign = match b[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (hours, minutes) = (number(&b[1..3])?, number(&b[4..6])?);
    if hours > 14 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3600 + minutes * 60))
}

fn number(digits: &[u8]) -> Option<i64> {
    digits.iter().try_fold(0i64, |acc, &d| d.is_ascii_digit().then(|| acc * 10 + i64::from(d - b'0')))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// Howard Hinnant's days_from_civil / civil_from_days (proleptic Gregorian, day 0 = 1970-01-01).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lima(text: &str) -> (String, String) {
        let lima = text.parse::<DateTime>().unwrap().to_lima();
        (lima.date_string(), lima.time_string())
    }

    #[test]
    fn converts_to_lima_wall_clock() {
        assert_eq!(lima("2026-10-15T10:30:00-05:00"), ("2026-10-15".into(), "10:30:00".into()));
        assert_eq!(lima("2026-10-15T03:00:00Z"), ("2026-10-14".into(), "22:00:00".into()));
        assert_eq!(lima("2024-02-29T23:59:59+01:00"), ("2024-02-29".into(), "17:59:59".into()));
        assert_eq!(lima("2027-01-01T04:59:59+00:00"), ("2026-12-31".into(), "23:59:59".into()));
    }

    #[test]
    fn calendar_dates_round_trip_and_compact() {
        let date: Date = "2024-02-29".parse().unwrap();
        assert_eq!((date.date_string(), date.compact()), ("2024-02-29".into(), "20240229".into()));
        assert!("2026-10-14".parse::<Date>().unwrap() < "2026-10-15".parse::<Date>().unwrap());
        for text in ["2026-02-29", "2026-13-01", "2026-10-1", "2026-10-15T00:00:00-05:00", "20261015", ""] {
            assert!(text.parse::<Date>().is_err(), "{text}");
        }
    }

    #[test]
    fn rejects_dates_without_offset_or_out_of_range() {
        for text in ["2026-10-15", "2026-10-15T10:30:00", "2026-10-15T10:30:00.5-05:00", "2026-02-30T00:00:00-05:00", "2026-10-15 10:30:00-05:00"] {
            assert!(text.parse::<DateTime>().is_err(), "{text}");
        }
    }
}
