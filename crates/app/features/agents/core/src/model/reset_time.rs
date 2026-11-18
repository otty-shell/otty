//! When a usage window resets or a credits pool renews, written with its
//! exact time in the user's time zone (spec Decision 10), and how the
//! dashboard writes a date, a clock time and a length of time.

use std::time::SystemTime;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use otty_agents::LocalNow;
use otty_app_i18n::{self as i18n, Key};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
/// The farthest day ahead still named by its day of the week; a later one
/// is named by its date, so no day name stands for two dates.
const LAST_DAY_BY_NAME: i64 = 6;

/// The reset at `resets_at` seen at `now`, in the user's `time_zone`, or
/// in UTC when it is not known:
/// "Resets in 2 h 10 min, at 15:30" within a day, "Resets on Thursday at
/// 09:00" within six days, "Resets on Nov 23 at 09:00" later. A reset
/// within the minute, or already past, is "Resets now".
pub(crate) fn resets_text(
    now: SystemTime,
    resets_at: SystemTime,
    time_zone: Option<&TimeZone>,
) -> String {
    let seconds_left = resets_at
        .duration_since(now)
        .map_or(0, |left| left.as_secs());
    if seconds_left < MINUTE {
        return i18n::t(Key::AgentsResetsNow).to_string();
    }

    let local = local_now(now, time_zone);
    let time = clock_text(&local, resets_at, time_zone.is_some());

    if seconds_left < DAY {
        return i18n::filled(
            Key::TplAgentsResetsInAt,
            &[("duration", &duration_text(seconds_left)), ("time", &time)],
        );
    }

    let date = local.date_of(resets_at);
    if local.days_until(resets_at) <= LAST_DAY_BY_NAME {
        return i18n::filled(
            Key::TplAgentsResetsOnDay,
            &[
                (
                    "day",
                    i18n::agents_weekday(date.weekday().to_monday_one_offset()),
                ),
                ("time", &time),
            ],
        );
    }

    i18n::filled(
        Key::TplAgentsResetsOnDate,
        &[("date", &date_text(date)), ("time", &time)],
    )
}

/// The clock time of `time` in the time zone of `local`, such as "15:30";
/// with "UTC" after it when the user's time zone is not known.
pub(crate) fn clock_text(
    local: &LocalNow,
    time: SystemTime,
    is_local: bool,
) -> String {
    let clock = local.time_of_day(time).strftime("%H:%M").to_string();

    if is_local {
        return clock;
    }
    i18n::filled(Key::TplAgentsTimeUtc, &[("time", &clock)])
}

/// The date and the clock time of `time`, such as "Nov 16, 2026, 10:00",
/// in the user's `time_zone`, or in UTC when it is not known.
pub(crate) fn date_time_text(
    time: SystemTime,
    time_zone: Option<&TimeZone>,
) -> String {
    let local = local_now(time, time_zone);
    let date = local.date_of(time);
    let month_day = date_text(date);
    let year = date.year().to_string();
    let clock = clock_text(&local, time, time_zone.is_some());

    i18n::filled(
        Key::TplAgentsDateTime,
        &[("date", &month_day), ("year", &year), ("time", &clock)],
    )
}

/// A date as the dashboard writes it, its month and its day in the user's
/// language, such as "Nov 13".
pub(crate) fn date_text(date: Date) -> String {
    let day = date.day().to_string();

    i18n::filled(
        Key::TplAgentsMonthDay,
        &[("month", i18n::agents_month(date.month())), ("day", &day)],
    )
}

/// How long `seconds` are, in whole hours and minutes: "2 h 10 min",
/// "3 h" or "40 min".
pub(crate) fn duration_text(seconds: u64) -> String {
    let hours = seconds / HOUR;
    let minutes = seconds % HOUR / MINUTE;

    match (hours, minutes) {
        (0, minutes) => i18n::with_count(Key::TplMinutes, minutes),
        (hours, 0) => i18n::with_count(Key::TplHours, hours),
        (hours, minutes) => i18n::filled(
            Key::TplAgentsDurationHoursMinutes,
            &[
                ("hours", &format!("{hours}")),
                ("minutes", &format!("{minutes}")),
            ],
        ),
    }
}

/// The time `now` in the user's `time_zone`, or in UTC when it is not
/// known.
fn local_now(now: SystemTime, time_zone: Option<&TimeZone>) -> LocalNow {
    let time_zone = time_zone.cloned().unwrap_or(TimeZone::UTC);

    LocalNow::new(now, time_zone)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::testing::{berlin, moscow};

    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    /// Monday, 2026-11-16, 10:00 UTC: 13:00 in Moscow.
    fn monday() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_794_823_200)
    }

    fn after(seconds: u64) -> SystemTime {
        monday() + Duration::from_secs(seconds)
    }

    fn resets_in(duration: &str, time: &str) -> String {
        i18n::filled(
            Key::TplAgentsResetsInAt,
            &[("duration", duration), ("time", time)],
        )
    }

    fn hours_minutes(hours: &str, minutes: &str) -> String {
        i18n::filled(
            Key::TplAgentsDurationHoursMinutes,
            &[("hours", hours), ("minutes", minutes)],
        )
    }

    fn resets_on_day(weekday: i8, time: &str) -> String {
        i18n::filled(
            Key::TplAgentsResetsOnDay,
            &[("day", i18n::agents_weekday(weekday)), ("time", time)],
        )
    }

    fn in_utc(time: &str) -> String {
        i18n::filled(Key::TplAgentsTimeUtc, &[("time", time)])
    }

    #[test]
    fn within_a_day_the_reset_says_how_long_and_at_what_time() {
        let in_moscow = |seconds: u64| {
            resets_text(monday(), after(seconds), Some(&moscow()))
        };

        assert_eq!(
            in_moscow(2 * HOUR + 10 * MINUTE + 59),
            resets_in(&hours_minutes("2", "10"), "15:10")
        );
        assert_eq!(
            in_moscow(3 * HOUR),
            resets_in(&i18n::with_count(Key::TplHours, 3), "16:00")
        );
        assert_eq!(
            in_moscow(40 * MINUTE),
            resets_in(&i18n::with_count(Key::TplMinutes, 40), "13:40")
        );
        assert_eq!(
            in_moscow(DAY - MINUTE),
            resets_in(&hours_minutes("23", "59"), "12:59"),
            "tomorrow, but within a day"
        );
    }

    #[test]
    fn within_six_days_the_reset_names_the_day_of_the_week() {
        let thursday_nine = after(3 * DAY - 4 * HOUR);
        let sunday_late = after(6 * DAY + 10 * HOUR);

        assert_eq!(
            resets_text(monday(), thursday_nine, Some(&moscow())),
            resets_on_day(4, "09:00")
        );
        assert_eq!(
            resets_text(monday(), sunday_late, Some(&moscow())),
            resets_on_day(7, "23:00")
        );
    }

    #[test]
    fn later_the_reset_names_the_date() {
        let next_monday_nine = after(7 * DAY - 4 * HOUR);

        let text = resets_text(monday(), next_monday_nine, Some(&moscow()));

        let date = i18n::filled(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", "23")],
        );
        assert_eq!(
            text,
            i18n::filled(
                Key::TplAgentsResetsOnDate,
                &[("date", &date), ("time", "09:00")]
            )
        );
    }

    #[test]
    fn a_reset_after_the_end_of_summer_time_is_written_with_its_offset() {
        // Saturday 2026-10-24, 12:00 UTC: 14:00 in Berlin, on summer time.
        let saturday = monday() - Duration::from_secs(23 * DAY - 2 * HOUR);
        // Sunday, 22:30 UTC, after summer time ended: 23:30 in Berlin, not
        // 00:30 on Monday as with the summer offset of now.
        let sunday_late =
            saturday + Duration::from_secs(34 * HOUR + 30 * MINUTE);

        assert_eq!(
            resets_text(saturday, sunday_late, Some(&berlin())),
            resets_on_day(7, "23:30")
        );
    }

    #[test]
    fn a_reset_within_the_minute_or_past_is_now() {
        assert_eq!(
            resets_text(monday(), after(30), Some(&moscow())),
            i18n::t(Key::AgentsResetsNow)
        );
        assert_eq!(
            resets_text(
                monday(),
                monday() - Duration::from_secs(HOUR),
                Some(&moscow())
            ),
            i18n::t(Key::AgentsResetsNow)
        );
    }

    #[test]
    fn with_no_known_time_zone_the_times_are_in_utc_and_say_so() {
        let soon = after(2 * HOUR + 10 * MINUTE);
        let thursday = after(3 * DAY - 4 * HOUR);

        assert_eq!(
            resets_text(monday(), soon, None),
            resets_in(&hours_minutes("2", "10"), &in_utc("12:10"))
        );
        assert_eq!(
            resets_text(monday(), thursday, None),
            resets_on_day(4, &in_utc("06:00"))
        );
    }

    #[test]
    fn a_date_is_written_as_its_month_and_day() {
        assert_eq!(
            date_text(jiff::civil::date(2026, 3, 7)),
            i18n::filled(
                Key::TplAgentsMonthDay,
                &[("month", i18n::agents_month(3)), ("day", "7")]
            )
        );
    }
}
