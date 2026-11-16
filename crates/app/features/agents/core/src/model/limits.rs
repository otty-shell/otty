use std::time::SystemTime;

use otty_agents::{
    Billing, Credits, ExtraUsage, LimitWindow, SourceError, SourceFailure,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::limits::ProfileLimits;
use super::failure_reasons;
use super::numbers::{credits_text, money_text};

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// The limits block of a subscription or credits profile on its dashboard
/// (spec Decisions 5 and 9): each usage window with its bar, the extra
/// usage, or the credits, with where they came from. The last good answer
/// stays, greyed, when a later read fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LimitsViewModel {
    /// The source that answered and how long ago, such as "From usage
    /// request, 2 min ago".
    pub(crate) from: Option<String>,
    /// One bar for each usage window, then the extra usage, in the order
    /// of the answer.
    pub(crate) windows: Vec<WindowViewModel>,
    /// The credits of an account paid for by credits.
    pub(crate) credits: Option<CreditsViewModel>,
    /// Whether the numbers are an older answer, drawn greyed, because the
    /// latest read failed.
    pub(crate) is_stale: bool,
    /// How old the numbers are when the source read them from a record
    /// in a session file rather than told them as they are now.
    pub(crate) recorded: Option<String>,
    /// Why the latest read failed: the limits are not available, or could
    /// not be read again.
    pub(crate) failure: Option<String>,
    /// What to do when the login is the reason.
    pub(crate) log_in_hint: Option<&'static str>,
}

impl LimitsViewModel {
    /// The limits of a profile at `now`; `None` while there is nothing to
    /// draw: nothing was read yet, or the agent has no way to read limits.
    pub(crate) fn from_limits(
        limits: &ProfileLimits,
        now: SystemTime,
    ) -> Option<Self> {
        if !limits.has_sources() {
            return None;
        }

        let failures = limits.failures();
        let log_in_hint = is_login_reason(failures)
            .then(|| i18n::t(Key::AgentsLimitsLogInAgain));

        let Some((answer, read_at)) = limits.last_good() else {
            let failures = failures?;
            return Some(Self {
                from: None,
                windows: Vec::new(),
                credits: None,
                is_stale: false,
                recorded: None,
                failure: Some(i18n::with_error(
                    Key::TplAgentsLimitsNotAvailable,
                    &failure_reasons(failures),
                )),
                log_in_hint,
            });
        };

        let value = answer.value();
        let (windows, credits) = match value.billing() {
            Billing::UsageWindows(usage) => {
                let mut windows: Vec<WindowViewModel> = usage
                    .windows()
                    .iter()
                    .map(|window| window_bar(window, now))
                    .collect();
                windows.extend(usage.extra_usage().map(extra_usage_bar));
                (windows, None)
            },
            Billing::Credits(credits) => {
                (Vec::new(), Some(credits_window(credits, now)))
            },
            _ => (Vec::new(), None),
        };

        Some(Self {
            from: Some(from_text(answer.source(), now, read_at)),
            windows,
            credits,
            is_stale: failures.is_some(),
            recorded: value.recorded_at().map(|recorded_at| {
                i18n::agents_limits_recorded(&age_text(now, recorded_at))
            }),
            failure: failures.map(|failures| {
                i18n::with_error(
                    Key::TplAgentsLimitsReadFailed,
                    &failure_reasons(failures),
                )
            }),
            log_in_hint,
        })
    }
}

/// One usage window, or the extra usage, as a bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WindowViewModel {
    /// The name of the window, such as "5 hours".
    pub(crate) name: String,
    /// How much is used, such as "42 % used" or "$4.20 of $50".
    pub(crate) used: String,
    /// How full the bar is, from 0 to 100; `None` draws no bar.
    pub(crate) percent: Option<u8>,
    /// When the window starts again, such as "resets in 3 h".
    pub(crate) resets: Option<String>,
}

/// The credits of an account: the bar only when the pool is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreditsViewModel {
    /// The credits used of the pool, such as "1 240 of 5 000 used", when
    /// both are known.
    pub(crate) used: Option<String>,
    /// How much of the pool is used, from 0 to 100, when both are known.
    pub(crate) percent: Option<u8>,
    /// The credits left, such as "3 760 left", "Unlimited", or "Available"
    /// when the plan tells no number of them.
    pub(crate) left: Option<String>,
    /// When the pool renews, such as "resets in 3 d".
    pub(crate) renews: Option<String>,
    /// Why credits that are only available have no number: "This plan
    /// does not report a balance."
    pub(crate) note: Option<&'static str>,
}

/// The bar of a usage window at `now`: the percent is rounded and kept
/// between 0 and 100. The reset is counted from its time, or, for a source
/// that gives no time, is the agent's own text.
fn window_bar(window: &LimitWindow, now: SystemTime) -> WindowViewModel {
    let percent = window.percent_used().clamp(0.0, 100.0).round() as u8;

    WindowViewModel {
        name: window.name().to_string(),
        used: i18n::agents_template(
            Key::TplAgentsPercentUsed,
            &[("count", &format!("{percent}"))],
        ),
        percent: Some(percent),
        resets: match window.resets_at() {
            Some(resets_at) => Some(resets_text(now, resets_at)),
            None => window.reset_text().map(str::to_string),
        },
    }
}

/// The extra usage as a bar: the money spent of the limit, and the bar
/// only when both are known.
fn extra_usage_bar(extra: &ExtraUsage) -> WindowViewModel {
    let currency = extra.currency();
    let spent = extra.spent().map(|spent| money_text(spent, currency));
    let limit = extra.limit().map(|limit| money_text(limit, currency));

    let used = match (spent, limit) {
        (Some(spent), Some(limit)) => i18n::agents_template(
            Key::TplAgentsAmountOf,
            &[("amount", &spent), ("limit", &limit)],
        ),
        (Some(spent), None) => spent,
        (None, Some(limit)) => limit,
        (None, None) => String::new(),
    };

    WindowViewModel {
        name: i18n::t(Key::AgentsExtraUsage).to_string(),
        used,
        percent: share_of(extra.spent(), extra.limit()),
        resets: None,
    }
}

/// The credits: used of the pool and the bar only when both are known,
/// what is left, when the pool renews. Credits with no number are
/// "Available", with a note that the plan tells no balance (spec Decision
/// 9: show what is known, guess nothing).
fn credits_window(credits: &Credits, now: SystemTime) -> CreditsViewModel {
    let used = match (credits.used(), credits.pool()) {
        (Some(used), Some(pool)) => Some(i18n::agents_template(
            Key::TplAgentsCreditsUsedOf,
            &[("used", &credits_text(used)), ("pool", &credits_text(pool))],
        )),
        _ => None,
    };
    let left = if credits.is_unlimited() {
        Some(i18n::t(Key::AgentsCreditsUnlimited).to_string())
    } else if credits.is_only_available() {
        Some(i18n::t(Key::AgentsCreditsAvailable).to_string())
    } else {
        credits
            .left()
            .map(|left| i18n::agents_credits_left(&credits_text(left)))
    };

    CreditsViewModel {
        used,
        percent: share_of(credits.used(), credits.pool()),
        left,
        renews: credits
            .renews_at()
            .map(|renews_at| resets_text(now, renews_at)),
        note: credits
            .is_only_available()
            .then(|| i18n::t(Key::AgentsCreditsNoBalance)),
    }
}

/// How much of `whole` the `part` is, from 0 to 100; `None` unless both
/// are known and the whole is above zero.
fn share_of(part: Option<f64>, whole: Option<f64>) -> Option<u8> {
    let (part, whole) = (part?, whole?);
    if whole <= 0.0 {
        return None;
    }

    Some((part / whole * 100.0).clamp(0.0, 100.0).round() as u8)
}

/// Whether any of `failures` is a missing or expired login.
fn is_login_reason(failures: Option<&[SourceFailure]>) -> bool {
    failures.is_some_and(|failures| {
        failures
            .iter()
            .any(|failure| *failure.error() == SourceError::NotLoggedIn)
    })
}

/// When a window starts again, from `now`, in its largest whole unit:
/// "resets in 3 h". A time within the minute, or already past, is "resets
/// now".
fn resets_text(now: SystemTime, resets_at: SystemTime) -> String {
    let seconds = resets_at
        .duration_since(now)
        .map_or(0, |left| left.as_secs());

    let (key, unit) = match seconds {
        seconds if seconds < MINUTE => {
            return i18n::t(Key::AgentsResetsNow).to_string();
        },
        seconds if seconds < HOUR => (Key::TplAgentsResetsInMinutes, MINUTE),
        seconds if seconds < DAY => (Key::TplAgentsResetsInHours, HOUR),
        _ => (Key::TplAgentsResetsInDays, DAY),
    };

    i18n::with_count(key, seconds / unit)
}

/// The text of the source that answered and how long ago.
fn from_text(source: &str, now: SystemTime, read_at: SystemTime) -> String {
    i18n::agents_template(
        Key::TplAgentsLimitsFrom,
        &[("source", source), ("time", &age_text(now, read_at))],
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{Limits, UsageWindows};

    use super::*;
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        claude, codex, connected, five_hour_limits, limits_answer,
        limits_loaded, now, with_profiles,
    };

    const HOUR_LATER: Duration = Duration::from_secs(HOUR);

    /// Agents mode with Claude Code connected and its limits read at
    /// `read_at` from `sources`.
    fn read_limits(
        state: &mut AgentsState,
        sources: Vec<(&'static str, Result<Limits, SourceError>)>,
        read_at: SystemTime,
    ) {
        let _ = reduce(
            state,
            limits_loaded(&claude(), limits_answer(sources), read_at),
        );
    }

    fn limits_of(state: &AgentsState) -> Option<LimitsViewModel> {
        LimitsViewModel::from_limits(state.profiles()[0].limits(), now())
    }

    fn percent_used(percent: u64) -> String {
        i18n::agents_template(
            Key::TplAgentsPercentUsed,
            &[("count", &format!("{percent}"))],
        )
    }

    #[test]
    fn an_answer_gives_one_bar_for_each_window_with_its_source_and_age() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let five_hours =
            LimitWindow::new("5 hours", 41.6, Some(now() + HOUR_LATER));
        let weekly = LimitWindow::new("7 days", 103.4, None);
        let limits = Limits::new(
            Billing::UsageWindows(UsageWindows::new(
                vec![five_hours, weekly],
                None,
            )),
            Some(String::from("Plus")),
        );
        let two_minutes_ago = now() - Duration::from_secs(2 * MINUTE);
        read_limits(
            &mut state,
            vec![("usage request", Ok(limits))],
            two_minutes_ago,
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(
            vm,
            LimitsViewModel {
                from: Some(from_text("usage request", now(), two_minutes_ago)),
                windows: vec![
                    WindowViewModel {
                        name: String::from("5 hours"),
                        used: percent_used(42),
                        percent: Some(42),
                        resets: Some(i18n::with_count(
                            Key::TplAgentsResetsInHours,
                            1
                        )),
                    },
                    WindowViewModel {
                        name: String::from("7 days"),
                        used: percent_used(100),
                        percent: Some(100),
                        resets: None,
                    },
                ],
                credits: None,
                is_stale: false,
                recorded: None,
                failure: None,
                log_in_hint: None,
            }
        );
    }

    #[test]
    fn a_window_with_no_reset_time_shows_the_agents_own_reset_text() {
        let with_text = LimitWindow::new("5 hours", 29.0, None)
            .with_reset_text("Resets 5pm (Europe/Berlin)");
        let with_both =
            LimitWindow::new("7 days", 17.0, Some(now() + HOUR_LATER))
                .with_reset_text("Resets Nov 20, 10am (Europe/Berlin)");

        let text_bar = window_bar(&with_text, now());
        let both_bar = window_bar(&with_both, now());

        assert_eq!(
            text_bar.resets.as_deref(),
            Some("Resets 5pm (Europe/Berlin)")
        );
        assert_eq!(
            both_bar.resets,
            Some(i18n::with_count(Key::TplAgentsResetsInHours, 1))
        );
    }

    #[test]
    fn the_extra_usage_shows_its_money_and_a_bar_only_with_a_limit() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let extra =
            ExtraUsage::new(Some(4.2), Some(50.0), Some(String::from("USD")));
        let limits = Limits::new(
            Billing::UsageWindows(UsageWindows::new(Vec::new(), Some(extra))),
            None,
        );
        read_limits(&mut state, vec![("usage request", Ok(limits))], now());
        let with_limit = limits_of(&state).expect("the limits show");

        let spent_only =
            extra_usage_bar(&ExtraUsage::new(Some(4.2), None, None));

        assert_eq!(
            with_limit.windows,
            vec![WindowViewModel {
                name: i18n::t(Key::AgentsExtraUsage).to_string(),
                used: i18n::agents_template(
                    Key::TplAgentsAmountOf,
                    &[("amount", "$4.20"), ("limit", "$50.00")]
                ),
                percent: Some(8),
                resets: None,
            }]
        );
        assert_eq!(spent_only.used, "$4.20");
        assert_eq!(spent_only.percent, None);
    }

    #[test]
    fn numbers_from_a_session_file_say_how_old_they_are() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let half_an_hour_ago = now() - Duration::from_secs(30 * MINUTE);
        let recorded =
            five_hour_limits(42.0).with_recorded_at(half_an_hour_ago);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![("session file", Ok(recorded))]),
                now(),
            ),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(
            vm.recorded,
            Some(i18n::agents_limits_recorded(&i18n::with_count(
                Key::TplAgentsAgeMinutes,
                30
            )))
        );
        assert!(!vm.is_stale, "the read itself answered");
    }

    #[test]
    fn the_last_good_answer_is_kept_greyed_with_its_time_after_a_failure() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let ten_minutes_ago = now() - Duration::from_secs(10 * MINUTE);
        read_limits(
            &mut state,
            vec![("usage request", Ok(five_hour_limits(42.0)))],
            ten_minutes_ago,
        );
        read_limits(
            &mut state,
            vec![(
                "usage request",
                Err(SourceError::Network(String::from("timed out"))),
            )],
            now(),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(vm.windows.len(), 1);
        assert_eq!(vm.windows[0].percent, Some(42));
        assert!(vm.is_stale);
        assert_eq!(
            vm.from,
            Some(from_text("usage request", now(), ten_minutes_ago))
        );
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsLimitsReadFailed,
                "usage request: network error: timed out"
            ))
        );
        assert_eq!(vm.log_in_hint, None);
    }

    #[test]
    fn with_no_answer_ever_the_reasons_and_the_login_hint_show() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read_limits(
            &mut state,
            vec![
                ("usage request", Err(SourceError::NotLoggedIn)),
                (
                    "hidden terminal",
                    Err(SourceError::NotAvailable(String::from("no program"))),
                ),
            ],
            now(),
        );

        let vm = limits_of(&state).expect("the reasons show");

        assert!(vm.windows.is_empty());
        assert!(!vm.is_stale);
        assert_eq!(vm.from, None);
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsLimitsNotAvailable,
                "usage request: not logged in; hidden terminal: not available: no program"
            ))
        );
        assert_eq!(vm.log_in_hint, Some(i18n::t(Key::AgentsLimitsLogInAgain)));
    }

    #[test]
    fn nothing_shows_before_a_read_or_for_an_agent_with_no_sources() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ =
            reduce(&mut state, limits_loaded(&codex(), Err(Vec::new()), now()));

        assert_eq!(limits_of(&state), None);
        assert_eq!(
            LimitsViewModel::from_limits(state.profiles()[1].limits(), now()),
            None
        );
    }

    #[test]
    fn credits_with_a_known_pool_have_a_bar_and_a_balance_alone_has_none() {
        let renews_at = now() + Duration::from_secs(3 * DAY);
        let pool = credits_window(
            &Credits::new(
                Some(3760.0),
                Some(1240.0),
                Some(5000.0),
                Some(renews_at),
            ),
            now(),
        );
        let balance = credits_window(
            &Credits::new(Some(3760.0), None, None, None),
            now(),
        );
        let unlimited = credits_window(&Credits::unlimited(), now());

        assert_eq!(
            pool,
            CreditsViewModel {
                used: Some(i18n::agents_template(
                    Key::TplAgentsCreditsUsedOf,
                    &[("used", "1 240"), ("pool", "5 000")]
                )),
                percent: Some(25),
                left: Some(i18n::agents_credits_left("3 760")),
                renews: Some(i18n::with_count(Key::TplAgentsResetsInDays, 3)),
                note: None,
            }
        );
        assert_eq!(
            balance,
            CreditsViewModel {
                used: None,
                percent: None,
                left: Some(i18n::agents_credits_left("3 760")),
                renews: None,
                note: None,
            }
        );
        assert_eq!(
            unlimited.left.as_deref(),
            Some(i18n::t(Key::AgentsCreditsUnlimited))
        );
        assert_eq!(unlimited.note, None);
    }

    #[test]
    fn credits_with_no_number_are_available_with_a_note() {
        let renews_at = now() + Duration::from_secs(3 * DAY);
        let renewing = Credits::new(None, None, None, Some(renews_at));

        let available = credits_window(&Credits::available(), now());
        let renewing = credits_window(&renewing, now());

        assert_eq!(
            available,
            CreditsViewModel {
                used: None,
                percent: None,
                left: Some(i18n::t(Key::AgentsCreditsAvailable).to_string()),
                renews: None,
                note: Some(i18n::t(Key::AgentsCreditsNoBalance)),
            }
        );
        assert_eq!(
            renewing.renews,
            Some(i18n::with_count(Key::TplAgentsResetsInDays, 3))
        );
        assert_eq!(renewing.note, Some(i18n::t(Key::AgentsCreditsNoBalance)));
    }

    #[test]
    fn a_credits_answer_has_credits_and_no_windows() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let limits = Limits::new(
            Billing::Credits(Credits::new(Some(10.0), None, None, None)),
            None,
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![("app-server", Ok(limits))]),
                now(),
            ),
        );

        let vm = limits_of(&state).expect("the credits show");

        assert!(vm.windows.is_empty());
        assert_eq!(
            vm.credits.and_then(|credits| credits.left),
            Some(i18n::agents_credits_left("10"))
        );
    }

    #[test]
    fn the_reset_takes_its_largest_whole_unit() {
        let at = |seconds: u64| now() + Duration::from_secs(seconds);

        assert_eq!(resets_text(now(), at(30)), i18n::t(Key::AgentsResetsNow));
        assert_eq!(
            resets_text(now(), at(59 * MINUTE)),
            i18n::with_count(Key::TplAgentsResetsInMinutes, 59)
        );
        assert_eq!(
            resets_text(now(), at(5 * HOUR - 1)),
            i18n::with_count(Key::TplAgentsResetsInHours, 4)
        );
        assert_eq!(
            resets_text(now(), at(3 * DAY)),
            i18n::with_count(Key::TplAgentsResetsInDays, 3)
        );
        assert_eq!(
            resets_text(now(), now() - Duration::from_secs(HOUR)),
            i18n::t(Key::AgentsResetsNow)
        );
    }
}
