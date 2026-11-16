//! How long ago a session changed, as the session list shows it.

use std::time::SystemTime;

use otty_app_i18n::{self as i18n, Key};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// How long before `now` the time `then` was: "just now", "5 min ago",
/// "3 h ago", "2 d ago", "1 w ago", "4 mo ago", "2 y ago". A time after
/// `now` is "just now".
pub(crate) fn age_text(now: SystemTime, then: SystemTime) -> String {
    let seconds = now
        .duration_since(then)
        .map_or(0, |elapsed| elapsed.as_secs());

    let (key, unit) = match seconds {
        seconds if seconds < MINUTE => {
            return i18n::t(Key::AgentsAgeJustNow).to_string();
        },
        seconds if seconds < HOUR => (Key::TplAgentsAgeMinutes, MINUTE),
        seconds if seconds < DAY => (Key::TplAgentsAgeHours, HOUR),
        seconds if seconds < WEEK => (Key::TplAgentsAgeDays, DAY),
        seconds if seconds < MONTH => (Key::TplAgentsAgeWeeks, WEEK),
        seconds if seconds < YEAR => (Key::TplAgentsAgeMonths, MONTH),
        _ => (Key::TplAgentsAgeYears, YEAR),
    };

    i18n::with_count(key, seconds / unit)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn ago(seconds: u64) -> String {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * YEAR);

        age_text(now, now - Duration::from_secs(seconds))
    }

    #[test]
    fn each_age_takes_its_largest_whole_unit() {
        assert_eq!(ago(59), i18n::t(Key::AgentsAgeJustNow));
        assert_eq!(ago(MINUTE), i18n::with_count(Key::TplAgentsAgeMinutes, 1));
        assert_eq!(
            ago(HOUR - 1),
            i18n::with_count(Key::TplAgentsAgeMinutes, 59)
        );
        assert_eq!(ago(2 * HOUR), i18n::with_count(Key::TplAgentsAgeHours, 2));
        assert_eq!(ago(3 * DAY), i18n::with_count(Key::TplAgentsAgeDays, 3));
        assert_eq!(ago(2 * WEEK), i18n::with_count(Key::TplAgentsAgeWeeks, 2));
        assert_eq!(
            ago(4 * MONTH),
            i18n::with_count(Key::TplAgentsAgeMonths, 4)
        );
        assert_eq!(ago(2 * YEAR), i18n::with_count(Key::TplAgentsAgeYears, 2));
    }

    #[test]
    fn a_time_after_now_is_just_now() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(DAY);

        assert_eq!(
            age_text(now, now + Duration::from_secs(HOUR)),
            i18n::t(Key::AgentsAgeJustNow)
        );
    }
}
