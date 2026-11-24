//! `clock.local-offset-minutes`: how far the user's clock is from UTC at
//! a given time, in the system's time zone, so a plugin counts the days
//! and months of the user's own calendar (issue 22 of spec
//! 2026-11-23-accounts-apart-from-profiles).

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// Seconds in a minute.
const MINUTE_SECONDS: i32 = 60;

/// How many minutes the user's clock is ahead of UTC at `at_ms`, in the
/// time zone the system is set to (the `TZ` variable, or the zone of the
/// system); in UTC when it can not be read.
pub(crate) fn local_offset_minutes(at_ms: i64) -> i32 {
    let time_zone = match TimeZone::try_system() {
        Ok(time_zone) => time_zone,
        Err(error) => {
            log::warn!("the time zone could not be read, it is UTC: {error}");
            TimeZone::UTC
        },
    };

    offset_minutes_in(&time_zone, at_ms)
}

/// How many minutes `time_zone` is ahead of UTC at `at_ms`; 0 for a time
/// out of the range of dates.
fn offset_minutes_in(time_zone: &TimeZone, at_ms: i64) -> i32 {
    let Ok(at) = Timestamp::from_millisecond(at_ms) else {
        return 0;
    };

    time_zone.to_offset(at).seconds() / MINUTE_SECONDS
}

#[cfg(test)]
mod tests {
    use jiff::tz::{TimeZone, offset};

    use super::*;

    /// 2026-07-01 at 12:00 UTC, in the summer time of Berlin.
    const JULY_1_NOON: i64 = 1_782_907_200_000;

    /// 2026-11-20 at 12:00 UTC, in the winter time of Berlin.
    const NOV_20_NOON: i64 = 1_795_176_000_000;

    #[test]
    fn the_offset_is_that_of_the_time_zone() {
        assert_eq!(offset_minutes_in(&TimeZone::UTC, NOV_20_NOON), 0);
        assert_eq!(
            offset_minutes_in(&TimeZone::fixed(offset(3)), NOV_20_NOON),
            180
        );
        assert_eq!(
            offset_minutes_in(&TimeZone::fixed(offset(-5)), NOV_20_NOON),
            -300
        );
    }

    /// The offset is taken at the time asked about: a day of summer time
    /// keeps its summer offset.
    #[test]
    fn the_offset_follows_summer_time() {
        let berlin = TimeZone::posix("CET-1CEST,M3.5.0,M10.5.0/3")
            .expect("a POSIX time zone");

        assert_eq!(offset_minutes_in(&berlin, JULY_1_NOON), 120);
        assert_eq!(offset_minutes_in(&berlin, NOV_20_NOON), 60);
    }

    /// A time out of the range of dates is in UTC.
    #[test]
    fn a_time_out_of_range_is_in_utc() {
        let moscow = TimeZone::fixed(offset(3));

        assert_eq!(offset_minutes_in(&moscow, i64::MAX), 0);
    }
}
