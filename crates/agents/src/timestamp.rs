//! Reads the times the agents write: RFC 3339 text in their files, such as
//! `2026-11-16T10:20:30.123Z`, and counts of seconds or milliseconds since
//! 1970 in their answers and login files.

use std::time::{Duration, SystemTime};

use jiff::Timestamp;

/// The time written as RFC 3339 text, or `None` when the text is not one.
///
/// The text is a date, a time with optional fractions of a second, and the
/// offset from UTC, such as `Z` or `+02:00`. A text with no offset is not
/// read: it does not tell which moment it is.
pub(crate) fn parse_rfc3339(text: &str) -> Option<SystemTime> {
    let time: Timestamp = text.parse().ok()?;

    Some(SystemTime::from(time))
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::at;

    #[test]
    fn a_time_with_its_fraction_and_offset_is_read() {
        let expected = at(1_794_823_200) + Duration::from_millis(123);

        assert_eq!(parse_rfc3339("2026-11-16T10:00:00.123Z"), Some(expected));
        assert_eq!(
            parse_rfc3339("2026-11-16T12:00:00.123+02:00"),
            Some(expected)
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
            "２０２６-11-16T10:00:00Z",
        ] {
            assert_eq!(parse_rfc3339(text), None, "{text}");
        }
    }
}
