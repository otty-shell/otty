//! Reads the RFC 3339 times that engines send for volumes and networks, and
//! for the runs and log lines of a container.
//!
//! Containers and images come with Unix seconds; volumes and networks come
//! with text such as `2026-11-01T19:30:36+01:00`. Reading the text here gives
//! the app one kind of time for every resource.

use jiff::Timestamp;

/// Nanoseconds in one second.
pub(crate) const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// Seconds since the Unix epoch for an RFC 3339 time such as
/// `2026-11-01T19:30:36+01:00` or `2026-01-18T18:25:28.650179515-05:00`.
///
/// Fractions of a second are dropped. `None` when the text is not such a
/// time, or when it is before 1970: engines send Go's zero time
/// (`0001-01-01T00:00:00Z`) when they do not know.
pub(crate) fn unix_seconds(text: &str) -> Option<i64> {
    let time = known_time(text)?;

    Some(time.as_second())
}

/// Nanoseconds since the Unix epoch for an RFC 3339 time such as
/// `2026-11-03T17:46:23.493268440Z`, with its fraction of a second. `None`
/// in the same cases as [`unix_seconds`].
pub(crate) fn unix_nanos(text: &str) -> Option<i64> {
    let time = known_time(text)?;

    i64::try_from(time.as_nanosecond()).ok()
}

/// The time of an RFC 3339 text; `None` for other text and for a time
/// before 1970, which an engine sends when it does not know.
fn known_time(text: &str) -> Option<Timestamp> {
    let time: Timestamp = text.trim().parse().ok()?;

    (time >= Timestamp::UNIX_EPOCH).then_some(time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_time_is_read() {
        assert_eq!(unix_seconds("1970-01-01T00:00:00Z"), Some(0));
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
        ] {
            assert_eq!(unix_seconds(text), None, "{text}");
        }
    }
}
