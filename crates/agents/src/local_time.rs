//! The current time in the user's time zone, and the calendar dates the
//! tokens spent are counted by.
//!
//! The crate reads no environment, so the app passes in the current time
//! and the user's time zone. The offset from UTC is taken for each time on
//! its own: a day of summer time keeps its summer offset even when today is
//! in winter.

use std::time::SystemTime;

use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

/// The current time and the user's time zone, which together tell what
/// "today" is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalNow {
    now: SystemTime,
    time_zone: TimeZone,
}

impl LocalNow {
    /// The current time.
    pub fn now(&self) -> SystemTime {
        self.now
    }

    /// The time `now` in the user's `time_zone`.
    ///
    /// ```
    /// use std::time::SystemTime;
    ///
    /// use jiff::tz::TimeZone;
    /// use otty_agents::LocalNow;
    ///
    /// let now = LocalNow::new(SystemTime::UNIX_EPOCH, TimeZone::UTC);
    ///
    /// assert_eq!(now.today(), jiff::civil::date(1970, 1, 1));
    /// ```
    pub fn new(now: SystemTime, time_zone: TimeZone) -> Self {
        Self { now, time_zone }
    }

    /// The date of today in the user's time zone.
    pub fn today(&self) -> Date {
        self.date_of(self.now)
    }

    /// The date `time` falls on in the user's time zone.
    ///
    /// ```
    /// use std::time::{Duration, SystemTime};
    ///
    /// use jiff::tz::{TimeZone, offset};
    /// use otty_agents::LocalNow;
    ///
    /// let in_moscow =
    ///     LocalNow::new(SystemTime::UNIX_EPOCH, TimeZone::fixed(offset(3)));
    /// let late = SystemTime::UNIX_EPOCH + Duration::from_secs(22 * 3600);
    ///
    /// assert_eq!(in_moscow.date_of(late), jiff::civil::date(1970, 1, 2));
    /// ```
    pub fn date_of(&self, time: SystemTime) -> Date {
        self.time_zone.to_datetime(timestamp(time)).date()
    }

    /// The clock time of `time` in the user's time zone, such as 15:30.
    pub fn time_of_day(&self, time: SystemTime) -> Time {
        self.time_zone.to_datetime(timestamp(time)).time()
    }

    /// How many of the user's midnights are between now and `time`: 0 for
    /// later today, 1 for tomorrow; negative for a day before today.
    pub fn days_until(&self, time: SystemTime) -> i64 {
        self.today()
            .until(self.date_of(time))
            .map_or(0, |span| i64::from(span.get_days()))
    }

    /// The time `date` starts in the user's time zone.
    pub(crate) fn start_of(&self, date: Date) -> SystemTime {
        date.to_zoned(self.time_zone.clone())
            .map_or(SystemTime::UNIX_EPOCH, |start| start.timestamp().into())
    }
}

/// The date `count` days before `date`.
pub(crate) fn days_before(date: Date, count: i32) -> Date {
    Span::new()
        .try_days(count)
        .map_or(Date::MIN, |back| date.saturating_sub(back))
}

/// Whether `date` is one of the `count` days that end with `today`, today
/// included.
pub(crate) fn is_in_last_days(date: Date, today: Date, count: i32) -> bool {
    date > days_before(today, count)
}

/// Whether `date` is in the same month as `today`.
pub(crate) fn is_in_month_of(date: Date, today: Date) -> bool {
    date.first_of_month() == today.first_of_month()
}

/// `time` as a jiff timestamp. A time out of jiff's range, more than
/// 9999 years from year 0, is kept at its edge.
fn timestamp(time: SystemTime) -> Timestamp {
    let edge = if time < SystemTime::UNIX_EPOCH {
        Timestamp::MIN
    } else {
        Timestamp::MAX
    };

    Timestamp::try_from(time).unwrap_or(edge)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jiff::civil::{date, time};

    use super::*;
    use crate::testing::{NOW, at, berlin, hours_east, utc_now};

    /// 2026-10-24T12:00:00Z, a Saturday: summer time in Berlin, the day
    /// before it ends.
    const SUMMER_SATURDAY: u64 = NOW - 23 * 24 * 3600 + 2 * 3600;

    #[test]
    fn today_depends_on_the_time_zone() {
        let late_evening_utc = at(NOW + 13 * 3600);

        let in_utc = LocalNow::new(late_evening_utc, TimeZone::UTC);
        let in_moscow = LocalNow::new(late_evening_utc, hours_east(3));
        let in_new_york = LocalNow::new(at(NOW - 11 * 3600), hours_east(-5));

        assert_eq!(in_utc.today(), date(2026, 11, 16));
        assert_eq!(in_moscow.today(), date(2026, 11, 17));
        assert_eq!(in_new_york.today(), date(2026, 11, 15));
    }

    #[test]
    fn a_day_starts_at_local_midnight() {
        let now = LocalNow::new(at(NOW), hours_east(3));
        let today = now.today();

        let start = now.start_of(today);

        assert_eq!(start, at(NOW - 13 * 3600));
        assert_eq!(now.date_of(start), today);
        assert_eq!(now.date_of(at(NOW - 13 * 3600 - 1)), date(2026, 11, 15));
    }

    #[test]
    fn a_day_of_summer_time_starts_at_its_own_midnight() {
        let in_winter = LocalNow::new(at(NOW), berlin());

        let start = in_winter.start_of(date(2026, 10, 21));

        // Midnight of summer time (UTC+2) is 22:00 UTC the day before.
        assert_eq!(start, at(NOW - 26 * 24 * 3600 - 12 * 3600));
    }

    #[test]
    fn a_time_before_the_epoch_falls_on_an_earlier_day() {
        let before = SystemTime::UNIX_EPOCH - Duration::from_secs(1);

        assert_eq!(utc_now().date_of(before), date(1969, 12, 31));
        assert_eq!(
            utc_now().start_of(date(1969, 12, 31)),
            before - Duration::from_secs(86_399)
        );
    }

    #[test]
    fn the_date_and_the_clock_of_a_time_depend_on_the_time_zone() {
        let in_moscow = LocalNow::new(at(NOW), hours_east(3));
        let in_new_york = LocalNow::new(at(NOW), hours_east(-5));
        let late_evening = at(NOW + 13 * 3600 + 25 * 60);

        assert_eq!(in_moscow.date_of(late_evening), date(2026, 11, 17));
        assert_eq!(in_moscow.time_of_day(late_evening), time(2, 25, 0, 0));
        assert_eq!(in_new_york.date_of(late_evening), date(2026, 11, 16));
        assert_eq!(in_new_york.time_of_day(late_evening), time(18, 25, 0, 0));
    }

    #[test]
    fn each_time_has_the_offset_of_its_own_day() {
        let in_winter = LocalNow::new(at(NOW), berlin());
        // 22:15 UTC on 2026-10-20: 00:15 on the 21st on summer time.
        let summer_night = at(NOW - 26 * 24 * 3600 - 11 * 3600 - 45 * 60);

        assert_eq!(in_winter.date_of(summer_night), date(2026, 10, 21));
        assert_eq!(in_winter.time_of_day(summer_night), time(0, 15, 0, 0));
    }

    #[test]
    fn the_days_between_two_times_count_the_local_midnights_between_them() {
        let in_moscow = LocalNow::new(at(NOW), hours_east(3));

        assert_eq!(in_moscow.days_until(at(NOW + 10 * 3600)), 0);
        assert_eq!(in_moscow.days_until(at(NOW + 11 * 3600)), 1);
        assert_eq!(in_moscow.days_until(at(NOW + 6 * 24 * 3600)), 6);
    }

    #[test]
    fn the_days_until_a_time_after_the_end_of_summer_time_use_its_offset() {
        let in_summer = LocalNow::new(at(SUMMER_SATURDAY), berlin());
        // 22:30 UTC on Sunday 2026-10-25: 23:30 on winter time (UTC+1),
        // but 00:30 on Monday with the summer offset of now.
        let sunday_late = at(SUMMER_SATURDAY + 34 * 3600 + 30 * 60);

        assert_eq!(in_summer.days_until(sunday_late), 1);
        assert_eq!(in_summer.time_of_day(sunday_late), time(23, 30, 0, 0));
    }

    #[test]
    fn the_last_days_end_with_today() {
        let today = date(2026, 11, 16);

        assert_eq!(days_before(today, 29), date(2026, 10, 18));
        assert!(is_in_last_days(today, today, 1));
        assert!(!is_in_last_days(date(2026, 11, 15), today, 1));
        assert!(is_in_last_days(date(2026, 11, 10), today, 7));
        assert!(!is_in_last_days(date(2026, 11, 9), today, 7));
    }

    #[test]
    fn the_month_of_today_starts_on_its_first_day() {
        let today = date(2026, 11, 16);

        assert!(is_in_month_of(date(2026, 11, 1), today));
        assert!(is_in_month_of(today, today));
        assert!(!is_in_month_of(date(2026, 10, 31), today));
        assert!(!is_in_month_of(date(2025, 11, 16), today));
    }
}
