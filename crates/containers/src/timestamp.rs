//! Reads the RFC 3339 times that engines send for volumes and networks, and
//! for the runs and log lines of a container.
//!
//! Containers and images come with Unix seconds; volumes and networks come
//! with text such as `2026-11-01T19:30:36+01:00`. Reading the text here gives
//! the app one kind of time for every resource.

/// Nanoseconds in one second.
pub(crate) const NANOS_PER_SECOND: i64 = 1_000_000_000;
/// RFC 3339 times from the engines have at most this many fraction digits.
const FRACTION_DIGITS: usize = 9;

/// Seconds since the Unix epoch for an RFC 3339 time such as
/// `2026-11-01T19:30:36+01:00` or `2026-01-18T18:25:28.650179515-05:00`.
///
/// Fractions of a second are dropped. `None` when the text is not such a
/// time, or when it is before 1970: engines send Go's zero time
/// (`0001-01-01T00:00:00Z`) when they do not know.
pub(crate) fn unix_seconds(text: &str) -> Option<i64> {
    let (date, rest) = text.trim().split_once(['T', 't'])?;
    let (clock, offset) = split_offset(rest)?;
    let clock = clock.split_once('.').map_or(clock, |(whole, _)| whole);

    let mut date_parts = date.split('-');
    let year = number(date_parts.next()?, 4)?;
    let month = number(date_parts.next()?, 2)?;
    let day = number(date_parts.next()?, 2)?;
    let mut clock_parts = clock.split(':');
    let hour = number(clock_parts.next()?, 2)?;
    let minute = number(clock_parts.next()?, 2)?;
    let second = number(clock_parts.next()?, 2)?;
    if date_parts.next().is_some()
        || clock_parts.next().is_some()
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }

    let days = days_since_epoch(year, month, day);
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset;

    (seconds >= 0).then_some(seconds)
}

/// Nanoseconds since the Unix epoch for an RFC 3339 time such as
/// `2026-11-03T17:46:23.493268440Z`, with its fraction of a second. `None`
/// in the same cases as [`unix_seconds`].
pub(crate) fn unix_nanos(text: &str) -> Option<i64> {
    let seconds = unix_seconds(text)?;

    seconds
        .checked_mul(NANOS_PER_SECOND)?
        .checked_add(fraction_nanos(text))
}

/// Split `19:30:36.5+01:00` into the clock and the zone offset in seconds.
fn split_offset(rest: &str) -> Option<(&str, i64)> {
    if let Some(clock) = rest.strip_suffix(['Z', 'z']) {
        return Some((clock, 0));
    }

    let sign_at = rest.rfind(['+', '-'])?;
    let (clock, zone) = rest.split_at(sign_at);
    let sign = if zone.starts_with('-') { -1 } else { 1 };
    let (hours, minutes) = zone[1..].split_once(':')?;
    let offset = number(hours, 2)? * 3_600 + number(minutes, 2)? * 60;

    Some((clock, sign * offset))
}

/// The fraction of a second in an RFC 3339 time, in nanoseconds; 0 without
/// one. Digits past the ninth are dropped.
fn fraction_nanos(text: &str) -> i64 {
    let Some((_, after_dot)) = text.split_once('.') else {
        return 0;
    };

    let mut nanos = 0;
    let mut digits = after_dot.bytes().take_while(u8::is_ascii_digit);
    for _ in 0..FRACTION_DIGITS {
        let digit = digits.next().map_or(0, |digit| digit - b'0');
        nanos = nanos * 10 + i64::from(digit);
    }

    nanos
}

/// Read exactly `len` ASCII digits.
fn number(text: &str, len: usize) -> Option<i64> {
    if text.len() != len || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    text.parse().ok()
}

/// Days from 1970-01-01 to a date of the proleptic Gregorian calendar
/// (Howard Hinnant's `days_from_civil`).
fn days_since_epoch(year: i64, month: i64, day: i64) -> i64 {
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
    use super::*;

    #[test]
    fn utc_time_is_read() {
        assert_eq!(unix_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_seconds("2000-02-29T12:00:00Z"), Some(951_825_600));
    }

    #[test]
    fn positive_offset_is_subtracted() {
        assert_eq!(
            unix_seconds("2026-11-01T19:30:36+01:00"),
            Some(1_793_557_836)
        );
    }

    #[test]
    fn negative_offset_and_fraction_are_read() {
        assert_eq!(
            unix_seconds("2026-01-18T18:25:28.650179515-05:00"),
            Some(1_768_778_728)
        );
    }

    #[test]
    fn lower_case_separators_are_accepted() {
        assert_eq!(unix_seconds("2000-02-29t12:00:00z"), Some(951_825_600));
    }

    #[test]
    fn go_zero_time_is_missing() {
        assert_eq!(unix_seconds("0001-01-01T00:00:00Z"), None);
    }

    #[test]
    fn nanoseconds_keep_the_fraction_of_a_second() {
        assert_eq!(
            unix_nanos("2026-11-03T17:46:23.493268440Z"),
            Some(1_793_727_983_493_268_440)
        );
        assert_eq!(
            unix_nanos("2026-11-03T18:46:23.493268440+01:00"),
            Some(1_793_727_983_493_268_440)
        );
    }

    #[test]
    fn nanoseconds_read_a_short_or_missing_fraction() {
        assert_eq!(unix_nanos("1970-01-01T00:00:01.5Z"), Some(1_500_000_000));
        assert_eq!(unix_nanos("1970-01-01T00:00:01Z"), Some(1_000_000_000));
        assert_eq!(
            unix_nanos("2026-11-03T18:47:23.7959351+01:00"),
            Some(1_793_728_043_795_935_100)
        );
    }

    #[test]
    fn nanoseconds_of_a_zero_time_or_other_text_are_missing() {
        for text in ["0001-01-01T00:00:00Z", "", "tick-3", "2026-11-03"] {
            assert_eq!(unix_nanos(text), None, "{text}");
        }
    }

    #[test]
    fn text_that_is_not_a_time_is_missing() {
        for text in [
            "",
            "yesterday",
            "2026-11-01",
            "2026-11-01T19:30:36",
            "2026-13-01T19:30:36Z",
            "2026-11-32T19:30:36Z",
            "2026-11-01T24:00:00Z",
            "2026-11-01T19:60:00Z",
            "2026-11-01T19:30:36+1:00",
            "26-11-01T19:30:36Z",
            "2026-11-01T19:30:3xZ",
        ] {
            assert_eq!(unix_seconds(text), None, "{text}");
        }
    }
}
