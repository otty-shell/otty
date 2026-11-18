//! How long ago something happened, in words. Every feature counts an age
//! by the same rule; a line of text writes it with "ago", such as "5 min
//! ago", and the Age column of a table writes it short, such as "5 min".

use std::time::Duration;

use crate::{Key, t, with_count};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;
/// An age under this many days is counted in days, as the Docker CLI does
/// ("13 days ago"); a longer one in weeks.
const DAYS_BEFORE_WEEKS: u64 = 14;

/// The unit an age of a minute or more is counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl AgeUnit {
    /// The template of an age in this unit with "ago", such as "5 min ago".
    fn ago_key(self) -> Key {
        match self {
            Self::Minutes => Key::TplMinutesAgo,
            Self::Hours => Key::TplHoursAgo,
            Self::Days => Key::TplDaysAgo,
            Self::Weeks => Key::TplWeeksAgo,
            Self::Months => Key::TplMonthsAgo,
            Self::Years => Key::TplYearsAgo,
        }
    }

    /// The short template of an age in this unit, such as "5 min".
    fn short_key(self) -> Key {
        match self {
            Self::Minutes => Key::TplMinutes,
            Self::Hours => Key::TplHours,
            Self::Days => Key::TplDays,
            Self::Weeks => Key::TplWeeks,
            Self::Months => Key::TplMonths,
            Self::Years => Key::TplYears,
        }
    }
}

/// How long ago something happened, `elapsed` before now, for a line of
/// text: "just now", "5 min ago", "3 h ago", "13 d ago", "2 w ago", "4 mo
/// ago", "2 y ago".
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use otty_app_i18n::{Locale, set_locale, time_ago};
///
/// set_locale(Locale::En);
///
/// assert_eq!(time_ago(Duration::from_secs(5 * 60)), "5 min ago");
/// ```
pub fn time_ago(elapsed: Duration) -> String {
    match age_in_units(elapsed) {
        Some((unit, count)) => with_count(unit.ago_key(), count),
        None => t(Key::JustNow).to_string(),
    }
}

/// How old something is, `elapsed` since it was made, for the Age column
/// of a table: "< 1 min", "5 min", "3 h", "13 d", "2 w", "4 mo", "2 y".
///
/// # Examples
///
/// ```
/// use std::time::Duration;
///
/// use otty_app_i18n::{Locale, set_locale, short_age};
///
/// set_locale(Locale::En);
///
/// assert_eq!(short_age(Duration::from_secs(3 * 60 * 60)), "3 h");
/// ```
pub fn short_age(elapsed: Duration) -> String {
    match age_in_units(elapsed) {
        Some((unit, count)) => with_count(unit.short_key(), count),
        None => t(Key::UnderAMinute).to_string(),
    }
}

/// The largest unit that fits in `elapsed` and the whole number of it;
/// `None` under a minute.
fn age_in_units(elapsed: Duration) -> Option<(AgeUnit, u64)> {
    let seconds = elapsed.as_secs();

    let (unit, length) = match seconds {
        seconds if seconds < MINUTE => return None,
        seconds if seconds < HOUR => (AgeUnit::Minutes, MINUTE),
        seconds if seconds < DAY => (AgeUnit::Hours, HOUR),
        seconds if seconds < DAYS_BEFORE_WEEKS * DAY => (AgeUnit::Days, DAY),
        seconds if seconds < MONTH => (AgeUnit::Weeks, WEEK),
        seconds if seconds < YEAR => (AgeUnit::Months, MONTH),
        _ => (AgeUnit::Years, YEAR),
    };

    Some((unit, seconds / length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Locale, fill_in, text_in};

    fn units(seconds: u64) -> Option<(AgeUnit, u64)> {
        age_in_units(Duration::from_secs(seconds))
    }

    #[test]
    fn an_age_under_a_minute_has_no_unit() {
        assert_eq!(units(0), None);
        assert_eq!(units(59), None);
    }

    #[test]
    fn each_age_takes_its_largest_whole_unit() {
        assert_eq!(units(MINUTE), Some((AgeUnit::Minutes, 1)));
        assert_eq!(units(HOUR - 1), Some((AgeUnit::Minutes, 59)));
        assert_eq!(units(2 * HOUR), Some((AgeUnit::Hours, 2)));
        assert_eq!(units(DAY - 1), Some((AgeUnit::Hours, 23)));
        assert_eq!(units(25 * HOUR), Some((AgeUnit::Days, 1)));
        assert_eq!(units(3 * DAY), Some((AgeUnit::Days, 3)));
        assert_eq!(units(29 * DAY), Some((AgeUnit::Weeks, 4)));
        assert_eq!(units(30 * DAY), Some((AgeUnit::Months, 1)));
        assert_eq!(units(4 * MONTH), Some((AgeUnit::Months, 4)));
        assert_eq!(units(364 * DAY), Some((AgeUnit::Months, 12)));
        assert_eq!(units(365 * DAY), Some((AgeUnit::Years, 1)));
        assert_eq!(units(3 * YEAR), Some((AgeUnit::Years, 3)));
    }

    /// Up to 13 days an age is in days, as the Docker CLI writes it.
    #[test]
    fn days_turn_into_weeks_at_two_weeks() {
        assert_eq!(units(13 * DAY), Some((AgeUnit::Days, 13)));
        assert_eq!(units(14 * DAY), Some((AgeUnit::Weeks, 2)));
    }

    #[test]
    fn an_age_is_written_with_ago_in_a_line_and_short_in_a_column() {
        let five_minutes = Duration::from_secs(5 * MINUTE);

        assert_eq!(time_ago(five_minutes), with_count(Key::TplMinutesAgo, 5));
        assert_eq!(short_age(five_minutes), with_count(Key::TplMinutes, 5));
        assert_eq!(time_ago(Duration::from_secs(59)), t(Key::JustNow));
        assert_eq!(short_age(Duration::from_secs(59)), t(Key::UnderAMinute));
        assert_eq!(
            time_ago(Duration::from_secs(2 * YEAR)),
            with_count(Key::TplYearsAgo, 2)
        );
        assert_eq!(
            short_age(Duration::from_secs(2 * YEAR)),
            with_count(Key::TplYears, 2)
        );
    }

    #[test]
    fn the_age_texts_read_as_expected_in_english() {
        let english = |key: Key| fill_in(Locale::En, key, &[("count", "3")]);

        assert_eq!(text_in(Locale::En, Key::JustNow), "just now");
        assert_eq!(english(Key::TplMinutesAgo), "3 min ago");
        assert_eq!(english(Key::TplDaysAgo), "3 d ago");
        assert_eq!(text_in(Locale::En, Key::UnderAMinute), "< 1 min");
        assert_eq!(english(Key::TplHours), "3 h");
        assert_eq!(english(Key::TplWeeks), "3 w");
    }

    #[test]
    fn every_age_template_fills_its_count_in_every_locale() {
        let units = [
            AgeUnit::Minutes,
            AgeUnit::Hours,
            AgeUnit::Days,
            AgeUnit::Weeks,
            AgeUnit::Months,
            AgeUnit::Years,
        ];

        for locale in Locale::ALL {
            for unit in units {
                for key in [unit.ago_key(), unit.short_key()] {
                    let text = fill_in(locale, key, &[("count", "7")]);

                    assert!(
                        text.contains('7') && !text.contains('{'),
                        "{} {key:?}: {text}",
                        locale.tag()
                    );
                }
            }
        }
    }
}
