//! The name of a usage window in the user's language, such as "5 hours" or
//! "7 days · Opus".

use std::time::Duration;

use otty_agents::WindowId;
use otty_app_i18n::{self as i18n, Key};

/// Seconds in a minute.
const MINUTE: u64 = 60;
/// Seconds in an hour.
const HOUR: u64 = 60 * MINUTE;
/// Seconds in a day.
const DAY: u64 = 24 * HOUR;

/// The name of `window`: its length in whole days, else whole hours, else
/// minutes, and the model it counts alone after a dot, such as "7 days ·
/// Opus". A window of no length, or shorter than a minute, is "Limit".
pub(crate) fn window_name(window: &WindowId) -> String {
    let length = length_text(window.length());

    match window.model() {
        Some(model) => format!("{length} · {model}"),
        None => length,
    }
}

/// The length of a window, such as "5 hours", "1 day" or "30 min".
fn length_text(length: Option<Duration>) -> String {
    let seconds = length.map_or(0, |length| length.as_secs());

    match seconds {
        _ if seconds > 0 && seconds.is_multiple_of(DAY) => count_text(
            seconds / DAY,
            Key::AgentsWindowOneDay,
            Key::TplAgentsWindowDays,
        ),
        _ if seconds > 0 && seconds.is_multiple_of(HOUR) => count_text(
            seconds / HOUR,
            Key::AgentsWindowOneHour,
            Key::TplAgentsWindowHours,
        ),
        _ if seconds >= MINUTE => {
            i18n::with_count(Key::TplMinutes, seconds / MINUTE)
        },
        _ => i18n::t(Key::AgentsWindowLimit).to_string(),
    }
}

/// `count` with the text of one, `one`, or of more, `many`.
fn count_text(count: u64, one: Key, many: Key) -> String {
    if count == 1 {
        return i18n::t(one).to_string();
    }

    i18n::with_count(many, count)
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::Locale;
    use otty_app_testing::alone::is_alone;

    use super::*;

    fn seconds(seconds: u64) -> WindowId {
        WindowId::new(Some(Duration::from_secs(seconds)))
    }

    #[test]
    fn a_window_is_named_by_its_length_and_its_model() {
        assert_eq!(window_name(&WindowId::hours(5)), "5 hours");
        assert_eq!(window_name(&WindowId::days(7)), "7 days");
        assert_eq!(
            window_name(&WindowId::days(7).for_model("Opus")),
            "7 days · Opus"
        );
        assert_eq!(window_name(&WindowId::hours(1)), "1 hour");
        assert_eq!(window_name(&WindowId::days(1)), "1 day");
        assert_eq!(window_name(&WindowId::days(2)), "2 days");
    }

    #[test]
    fn a_window_of_odd_length_is_named_in_minutes_and_one_of_none_is_a_limit() {
        assert_eq!(window_name(&seconds(1800)), "30 min");
        assert_eq!(window_name(&seconds(5400)), "90 min");
        assert_eq!(window_name(&seconds(30)), "Limit");
        assert_eq!(window_name(&seconds(0)), "Limit");
        assert_eq!(window_name(&WindowId::new(None)), "Limit");
    }

    /// It switches the language of the whole process, so it runs alone in
    /// a copy of the test program.
    #[test]
    fn a_window_is_named_in_the_language_of_the_app() {
        if !is_alone("a_window_is_named_in_the_language_of_the_app") {
            return;
        }

        i18n::set_locale(Locale::Ru);

        assert_eq!(window_name(&WindowId::hours(5)), "5 ч");
        assert_eq!(
            window_name(&WindowId::days(7).for_model("Opus")),
            "7 дн · Opus"
        );
        assert_eq!(window_name(&WindowId::new(None)), "Лимит");
    }
}
