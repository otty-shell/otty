//! Reads the times the agents write: RFC 3339 text in their files, such as
//! `2026-11-16T10:20:30.123Z`, and counts of seconds or milliseconds since
//! 1970 in their answers and login files.

use std::ops::Range;
use std::time::{Duration, SystemTime};

const SECONDS_IN_DAY: i64 = 24 * 60 * 60;

/// The time written as RFC 3339 text, or `None` when the text is not one.
///
/// The text is a date, a time with optional fractions of a second, and the
/// offset from UTC: `Z` or `+hh:mm` / `-hh:mm`.
pub(crate) fn parse_rfc3339(text: &str) -> Option<SystemTime> {
    let year = number(text, 0..4)?;
    let month = number(text, 5..7)?;
    let day = number(text, 8..10)?;
    let hour = number(text, 11..13)?;
    let minute = number(text, 14..16)?;
    let second = number(text, 17..19)?;
    let separators = [(4, b'-'), (7, b'-'), (13, b':'), (16, b':')];
    let bytes = text.as_bytes();

    let separators_match = separators
        .iter()
        .all(|(index, separator)| bytes.get(*index) == Some(separator));
    let date_time_split = matches!(bytes.get(10), Some(b'T' | b't' | b' '));
    let in_range = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    if !separators_match || !date_time_split || !in_range {
        return None;
    }

    let (nanos, zone) = fraction(text.get(19..)?)?;
    let offset = offset_seconds(zone)?;

    let days = days_since_epoch(year, month, day);
    let seconds =
        days * SECONDS_IN_DAY + hour * 3600 + minute * 60 + second - offset;
    let since_epoch = Duration::new(seconds.unsigned_abs(), nanos);

    if seconds >= 0 {
        SystemTime::UNIX_EPOCH.checked_add(since_epoch)
    } else {
        SystemTime::UNIX_EPOCH.checked_sub(since_epoch)
    }
}

/// The time `seconds` after 1970; `None` when it is too far to be a time.
pub(crate) fn time_from_seconds(seconds: u64) -> Option<SystemTime> {
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

/// The time `millis` milliseconds after 1970; `None` for a count that is
/// negative, not finite or too far to be a time.
pub(crate) fn time_from_millis(millis: f64) -> Option<SystemTime> {
    let since_epoch = Duration::try_from_secs_f64(millis / 1000.0).ok()?;

    SystemTime::UNIX_EPOCH.checked_add(since_epoch)
}

/// The earlier of two times; either may be missing.
pub(crate) fn earliest(
    a: Option<SystemTime>,
    b: Option<SystemTime>,
) -> Option<SystemTime> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        _ => a.or(b),
    }
}

/// The later of two times; either may be missing.
pub(crate) fn latest(
    a: Option<SystemTime>,
    b: Option<SystemTime>,
) -> Option<SystemTime> {
    a.max(b)
}

/// The digits at `range` of `text` as a number.
fn number(text: &str, range: Range<usize>) -> Option<i64> {
    let digits = text.get(range)?;

    if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    digits.parse().ok()
}

/// The nanoseconds of an optional `.123` fraction at the start of `text`,
/// and the rest of the text after it.
fn fraction(text: &str) -> Option<(u32, &str)> {
    let Some(after_dot) = text.strip_prefix('.') else {
        return Some((0, text));
    };

    let length = after_dot
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if length == 0 {
        return None;
    }

    let (digits, rest) = after_dot.split_at(length);
    let kept = &digits[..length.min(9)];
    let scale = 10_u32.pow(9 - u32::try_from(kept.len()).ok()?);
    let nanos = kept.parse::<u32>().ok()? * scale;

    Some((nanos, rest))
}

/// The offset from UTC in seconds: `Z`, `+hh:mm` or `-hh:mm`, and nothing
/// after it.
fn offset_seconds(zone: &str) -> Option<i64> {
    if zone == "Z" || zone == "z" {
        return Some(0);
    }

    let sign = match zone.as_bytes().first() {
        Some(b'+') => 1,
        Some(b'-') => -1,
        _ => return None,
    };
    if zone.len() != 6 || zone.as_bytes().get(3) != Some(&b':') {
        return None;
    }

    let hours = number(zone, 1..3)?;
    let minutes = number(zone, 4..6)?;

    Some(sign * (hours * 3600 + minutes * 60))
}

/// Days from 1970-01-01 to the date, in the proleptic Gregorian calendar.
///
/// This is the `days_from_civil` algorithm of Howard Hinnant:
/// <https://howardhinnant.github.io/date_algorithms.html#days_from_civil>.
pub(crate) fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::at;

    #[test]
    fn the_epoch_is_read() {
        assert_eq!(
            parse_rfc3339("1970-01-01T00:00:00Z"),
            Some(SystemTime::UNIX_EPOCH)
        );
    }

    #[test]
    fn dates_before_and_after_a_leap_day_are_read() {
        assert_eq!(
            parse_rfc3339("2000-03-01T00:00:00Z"),
            Some(at(951_868_800))
        );
        assert_eq!(
            parse_rfc3339("2024-03-01T00:00:00Z"),
            Some(at(1_709_251_200))
        );
        assert_eq!(
            parse_rfc3339("2024-02-29T12:00:00Z"),
            Some(at(1_709_251_200 - 12 * 3600))
        );
    }

    #[test]
    fn fractions_of_a_second_are_kept() {
        let expected = at(1_794_823_200) + Duration::from_millis(123);

        assert_eq!(parse_rfc3339("2026-11-16T10:00:00.123Z"), Some(expected));
    }

    #[test]
    fn an_offset_from_utc_is_applied() {
        assert_eq!(
            parse_rfc3339("2026-11-16T12:00:00+02:00"),
            Some(at(1_794_823_200))
        );
        assert_eq!(
            parse_rfc3339("2026-11-16T05:30:00-04:30"),
            Some(at(1_794_823_200))
        );
    }

    #[test]
    fn seconds_and_milliseconds_since_1970_are_read() {
        assert_eq!(time_from_seconds(1_794_823_200), Some(at(1_794_823_200)));
        assert_eq!(
            time_from_millis(1_794_823_200_000.0),
            Some(at(1_794_823_200))
        );
        assert_eq!(time_from_millis(-1.0), None);
        assert_eq!(time_from_millis(f64::NAN), None);
    }

    #[test]
    fn the_earliest_and_the_latest_skip_a_missing_time() {
        let (early, late) = (Some(at(10)), Some(at(20)));

        assert_eq!(earliest(late, early), early);
        assert_eq!(earliest(None, late), late);
        assert_eq!(latest(early, late), late);
        assert_eq!(latest(early, None), early);
        assert_eq!(earliest(None, None), None);
    }

    #[test]
    fn text_that_is_not_a_time_is_not_read() {
        for text in [
            "",
            "yesterday",
            "2026-11-16",
            "2026-11-16T10:00:00",
            "2026-13-01T10:00:00Z",
            "2026-11-16T25:00:00Z",
            "2026-11-16T10:00:00.Z",
            "2026-11-16T10:00:00Zjunk",
            "2026-11-16T10:00:00+0200",
            "２０２６-11-16T10:00:00Z",
        ] {
            assert_eq!(parse_rfc3339(text), None, "{text}");
        }
    }
}
