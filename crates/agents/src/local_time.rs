//! The current time in the user's time zone, and the calendar dates the
//! tokens spent are counted by.
//!
//! The crate reads no environment, so the app passes in the current time
//! and the user's offset from UTC. One offset is used for the whole count:
//! on the days around a change of summer time a record near midnight may
//! fall on the next or the previous day.

use std::time::{Duration, SystemTime};

use crate::timestamp::days_since_epoch;

const SECONDS_IN_DAY: i64 = 24 * 60 * 60;

/// The current time and the user's offset from UTC, which together tell
/// what "today" is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalNow {
    now: SystemTime,
    utc_offset_seconds: i32,
}

impl LocalNow {
    /// The current time.
    pub fn now(&self) -> SystemTime {
        self.now
    }

    /// The user's offset from UTC in seconds, east of UTC positive.
    pub fn utc_offset_seconds(&self) -> i32 {
        self.utc_offset_seconds
    }

    /// The time `now` in a time zone `utc_offset_seconds` east of UTC.
    ///
    /// ```
    /// use std::time::SystemTime;
    ///
    /// use otty_agents::LocalNow;
    ///
    /// let now = LocalNow::new(SystemTime::now(), 3 * 3600);
    ///
    /// assert_eq!(now.utc_offset_seconds(), 10_800);
    /// ```
    pub fn new(now: SystemTime, utc_offset_seconds: i32) -> Self {
        Self {
            now,
            utc_offset_seconds,
        }
    }

    /// The date of today in the user's time zone.
    pub fn today(&self) -> CalendarDate {
        CalendarDate::from_day_number(self.day_of(self.now))
    }

    /// The number of the day `time` falls on in the user's time zone,
    /// counted from 1970-01-01.
    pub(crate) fn day_of(&self, time: SystemTime) -> i64 {
        let seconds = seconds_since_epoch(time)
            .saturating_add(i64::from(self.utc_offset_seconds));

        seconds.div_euclid(SECONDS_IN_DAY)
    }

    /// The time the day numbered `day` starts in the user's time zone.
    pub(crate) fn start_of(&self, day: i64) -> SystemTime {
        let seconds = day
            .saturating_mul(SECONDS_IN_DAY)
            .saturating_sub(i64::from(self.utc_offset_seconds));
        let since_epoch = Duration::from_secs(seconds.unsigned_abs());

        let start = if seconds >= 0 {
            SystemTime::UNIX_EPOCH.checked_add(since_epoch)
        } else {
            SystemTime::UNIX_EPOCH.checked_sub(since_epoch)
        };

        start.unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

/// A date of the calendar: year, month from 1 to 12, day from 1 to 31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarDate {
    year: i32,
    month: u8,
    day: u8,
}

impl CalendarDate {
    /// The year, such as 2026.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// The month, from 1 for January to 12.
    pub fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    pub fn day(&self) -> u8 {
        self.day
    }

    /// A date from its parts. The parts are not checked.
    ///
    /// ```
    /// use otty_agents::CalendarDate;
    ///
    /// let date = CalendarDate::new(2026, 11, 16);
    ///
    /// assert_eq!(date.month(), 11);
    /// ```
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// The date of the day numbered `day`, counted from 1970-01-01.
    ///
    /// This is the `civil_from_days` algorithm of Howard Hinnant:
    /// <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>.
    pub(crate) fn from_day_number(day: i64) -> Self {
        let shifted = day + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era = (day_of_era - day_of_era / 1460
            + day_of_era / 36_524
            - day_of_era / 146_096)
            / 365;
        let day_of_year = day_of_era
            - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let month_from_march = (5 * day_of_year + 2) / 153;
        let day_of_month = day_of_year - (153 * month_from_march + 2) / 5 + 1;
        let month = if month_from_march < 10 {
            month_from_march + 3
        } else {
            month_from_march - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);

        Self {
            year: i32::try_from(year).unwrap_or(i32::MAX),
            month: u8::try_from(month).unwrap_or(1),
            day: u8::try_from(day_of_month).unwrap_or(1),
        }
    }

    /// The number of this day, counted from 1970-01-01.
    pub(crate) fn day_number(&self) -> i64 {
        days_since_epoch(
            i64::from(self.year),
            i64::from(self.month),
            i64::from(self.day),
        )
    }

    /// The first day of the month of this date.
    pub(crate) fn first_of_month(&self) -> Self {
        Self { day: 1, ..*self }
    }
}

/// Seconds from the epoch to `time`, negative before it.
fn seconds_since_epoch(time: SystemTime) -> i64 {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_secs()).unwrap_or(i64::MAX),
        Err(before) => {
            let duration = before.duration();
            let seconds =
                duration.as_secs() + u64::from(duration.subsec_nanos() > 0);
            -i64::try_from(seconds).unwrap_or(i64::MAX)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::at;

    /// 2026-11-16T10:00:00Z.
    const NOW: u64 = 1_794_823_200;

    #[test]
    fn today_depends_on_the_offset_from_utc() {
        let late_evening_utc = at(NOW + 13 * 3600);

        let in_utc = LocalNow::new(late_evening_utc, 0);
        let in_moscow = LocalNow::new(late_evening_utc, 3 * 3600);
        let in_new_york = LocalNow::new(at(NOW - 11 * 3600), -5 * 3600);

        assert_eq!(in_utc.today(), CalendarDate::new(2026, 11, 16));
        assert_eq!(in_moscow.today(), CalendarDate::new(2026, 11, 17));
        assert_eq!(in_new_york.today(), CalendarDate::new(2026, 11, 15));
    }

    #[test]
    fn a_day_starts_at_local_midnight() {
        let now = LocalNow::new(at(NOW), 3 * 3600);
        let today = now.day_of(at(NOW));

        let start = now.start_of(today);

        assert_eq!(start, at(NOW - 13 * 3600));
        assert_eq!(now.day_of(start), today);
        assert_eq!(now.day_of(at(NOW - 13 * 3600 - 1)), today - 1);
    }

    #[test]
    fn dates_and_day_numbers_turn_into_each_other() {
        let dates = [
            (CalendarDate::new(1970, 1, 1), 0),
            (CalendarDate::new(2024, 2, 29), 19_782),
            (CalendarDate::new(2026, 11, 16), 20_773),
            (CalendarDate::new(2026, 12, 31), 20_818),
            (CalendarDate::new(1969, 12, 31), -1),
        ];

        for (date, number) in dates {
            assert_eq!(date.day_number(), number, "{date:?}");
            assert_eq!(CalendarDate::from_day_number(number), date);
        }
    }

    #[test]
    fn a_time_before_the_epoch_falls_on_an_earlier_day() {
        let now = LocalNow::new(at(NOW), 0);
        let before = SystemTime::UNIX_EPOCH - Duration::from_secs(1);

        assert_eq!(now.day_of(before), -1);
        assert_eq!(now.start_of(-1), before - Duration::from_secs(86_399));
    }

    #[test]
    fn the_first_of_the_month_keeps_the_year_and_the_month() {
        let date = CalendarDate::new(2026, 11, 16);

        assert_eq!(date.first_of_month(), CalendarDate::new(2026, 11, 1));
    }
}
