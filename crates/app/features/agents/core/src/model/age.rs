//! How long ago a time was, as Agents mode writes it.

use std::time::SystemTime;

use otty_app_i18n as i18n;

/// How long before `now` the time `then` was, such as "5 min ago", by the
/// age rule every feature shares. A time after `now`, from a clock that
/// was set back, is "just now".
pub(crate) fn age_text(now: SystemTime, then: SystemTime) -> String {
    let elapsed = now.duration_since(then).unwrap_or_default();

    i18n::time_ago(elapsed)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_app_i18n::Key;

    use super::*;

    #[test]
    fn an_age_is_counted_from_then_to_now() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let then = now - Duration::from_secs(3 * 60 * 60);

        assert_eq!(age_text(now, then), i18n::with_count(Key::TplHoursAgo, 3));
    }

    #[test]
    fn a_time_after_now_is_just_now() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);

        assert_eq!(
            age_text(now, now + Duration::from_secs(60 * 60)),
            i18n::t(Key::JustNow)
        );
    }
}
