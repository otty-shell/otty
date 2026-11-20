use std::time::SystemTime;

use jiff::tz::TimeZone;
use otty_agents::{
    Billing, Credits, ExtraUsage, LimitWindow, Percent, SourceError,
    SourceFailure,
};
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::failure_reasons;
use super::super::limits::ProfileLimits;
use super::age::age_text;
use super::kept_answer::KeptAnswer;
use super::numbers::{credits_text, money_text};
use super::reset_time::resets_text;
use super::window_name::window_name;

/// The Usage block of a subscription or credits profile on its page
/// (spec Decisions 5, 9 and 10): each usage window with its bar, the extra
/// usage, or the credits, with when they were read. The last good answer
/// stays, greyed, when a later read fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LimitsViewModel {
    /// When the numbers were read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// One bar for each usage window, then the extra usage, in the order
    /// of the answer.
    pub(crate) windows: Vec<UsageRowViewModel>,
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
    /// The limits of a profile at `now`, with the reset times in the
    /// user's `time_zone`, or in UTC when it is not known; `None` while
    /// there is nothing to draw: nothing was read yet, or the agent has no
    /// way to read limits.
    pub(crate) fn from_limits(
        limits: &ProfileLimits,
        now: SystemTime,
        time_zone: Option<&TimeZone>,
    ) -> Option<Self> {
        if !limits.has_sources() {
            return None;
        }

        let failures = limits.failures();
        let log_in_hint = is_login_reason(failures)
            .then(|| i18n::t(Key::AgentsLimitsLogInAgain));

        let (value, read_at, failures) =
            match KeptAnswer::new(limits.last_good(), failures) {
                KeptAnswer::Nothing => return None,
                KeptAnswer::OnlyFailure(failures) => {
                    return Some(Self {
                        updated: None,
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
                },
                KeptAnswer::Answer {
                    value,
                    read_at,
                    failure,
                } => (value.value(), read_at, failure),
            };

        let (windows, credits) = match value.billing() {
            Billing::UsageWindows(usage) => {
                let mut windows: Vec<UsageRowViewModel> = usage
                    .windows()
                    .iter()
                    .map(|window| window_bar(window, now, time_zone))
                    .collect();
                windows.extend(usage.extra_usage().map(extra_usage_bar));
                (windows, None)
            },
            Billing::Credits(credits) => {
                (Vec::new(), Some(credits_window(credits, now, time_zone)))
            },
            _ => (Vec::new(), None),
        };

        Some(Self {
            updated: Some(updated_text(now, read_at)),
            windows,
            credits,
            is_stale: failures.is_some(),
            recorded: value.recorded_at().map(|recorded_at| {
                i18n::filled(
                    Key::TplAgentsLimitsRecorded,
                    &[("time", &age_text(now, recorded_at))],
                )
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

/// One row of the Usage block: a usage window or the extra usage of a
/// subscription, or the money of an API key, its name on the left and its
/// value on the right, with a bar when it has a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageRowViewModel {
    /// The name of the row, such as "5 hours" or "Today".
    pub(crate) name: String,
    /// How much is used, such as "42 % used", "$4.20 of $50" or "$12.40".
    pub(crate) used: String,
    /// How full the bar is, from 0 to 100; `None` draws no bar.
    pub(crate) percent: Option<u8>,
    /// When the window starts again, such as "Resets in 2 h 10 min, at
    /// 15:30".
    pub(crate) resets: Option<String>,
}

/// The credits of an account, ready to draw: the value on the right of
/// the Credits row, the bar only when the pool is known, the muted line
/// under it and the note of credits with no number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreditsViewModel {
    /// The credits used of the pool, such as "1 240 of 5 000 used", when
    /// both are known; else what is left, such as "3 760 left",
    /// "Unlimited" or "Available".
    pub(crate) value: String,
    /// How much of the pool is used, from 0 to 100, when both are known.
    pub(crate) percent: Option<u8>,
    /// What is left and when the pool renews, such as "3 760 left ·
    /// Resets on Thursday at 09:00", leaving out what the value already
    /// says.
    pub(crate) line: Option<String>,
    /// Why credits that are only available have no number: "This plan
    /// does not report a balance."
    pub(crate) note: Option<&'static str>,
}

/// When the numbers read at `read_at` were read, from `now`: "Updated 2
/// min ago".
pub(crate) fn updated_text(now: SystemTime, read_at: SystemTime) -> String {
    i18n::filled(Key::TplAgentsUpdated, &[("time", &age_text(now, read_at))])
}

/// The credits left, such as "3 760 left", or that they have no limit;
/// `None` when the source told neither.
pub(crate) fn credits_left(credits: &Credits) -> Option<String> {
    if credits.is_unlimited() {
        return Some(i18n::t(Key::AgentsCreditsUnlimited).to_string());
    }

    let left = credits.left()?;
    Some(i18n::filled(
        Key::TplAgentsCreditsLeft,
        &[("amount", &credits_text(left))],
    ))
}

/// The bar of a usage window at `now`: the percent is rounded and kept
/// between 0 and 100. The reset is written from its time in the user's
/// `time_zone`, or, for a source that gives no time,
/// is the agent's own text.
fn window_bar(
    window: &LimitWindow,
    now: SystemTime,
    time_zone: Option<&TimeZone>,
) -> UsageRowViewModel {
    let percent = window.percent_used().rounded();

    UsageRowViewModel {
        name: window_name(window.id()),
        used: i18n::with_count(Key::TplAgentsPercentUsed, u64::from(percent)),
        percent: Some(percent),
        resets: match window.resets_at() {
            Some(resets_at) => Some(resets_text(now, resets_at, time_zone)),
            None => window.reset_text().map(str::to_string),
        },
    }
}

/// The extra usage as a bar: the money spent of the limit, and the bar
/// only when both are known.
fn extra_usage_bar(extra: &ExtraUsage) -> UsageRowViewModel {
    let currency = extra.currency();
    let spent = extra.spent().map(|spent| money_text(spent, currency));
    let limit = extra.limit().map(|limit| money_text(limit, currency));

    let used = match (spent, limit) {
        (Some(spent), Some(limit)) => i18n::filled(
            Key::TplAgentsAmountOf,
            &[("amount", &spent), ("limit", &limit)],
        ),
        (Some(spent), None) => spent,
        (None, Some(limit)) => limit,
        (None, None) => String::new(),
    };

    UsageRowViewModel {
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
fn credits_window(
    credits: &Credits,
    now: SystemTime,
    time_zone: Option<&TimeZone>,
) -> CreditsViewModel {
    let used = match (credits.used(), credits.pool()) {
        (Some(used), Some(pool)) => Some(i18n::filled(
            Key::TplAgentsCreditsUsedOf,
            &[("used", &credits_text(used)), ("pool", &credits_text(pool))],
        )),
        _ => None,
    };
    let left = credits_left(credits).or_else(|| {
        credits
            .is_only_available()
            .then(|| i18n::t(Key::AgentsCreditsAvailable).to_string())
    });
    let renews = credits
        .renews_at()
        .map(|renews_at| resets_text(now, renews_at, time_zone));

    let value = used.or_else(|| left.clone()).unwrap_or_default();
    let line = match (left, renews) {
        (Some(left), Some(renews)) => Some(i18n::filled(
            Key::TplAgentsCreditsLeftRenews,
            &[("left", &left), ("renews", &renews)],
        )),
        (Some(left), None) => Some(left),
        (None, renews) => renews,
    };

    CreditsViewModel {
        percent: share_of(credits.used(), credits.pool()),
        line: line.filter(|line| *line != value),
        value,
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

    Some(Percent::new(part / whole * 100.0).rounded())
}

/// Whether any of `failures` is a login the user has to renew. A login
/// the agent renews by itself ([`SourceError::LoginExpired`]) is not.
fn is_login_reason(failures: Option<&[SourceFailure]>) -> bool {
    failures.is_some_and(|failures| {
        failures
            .iter()
            .any(|failure| *failure.error() == SourceError::NotLoggedIn)
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{Limits, SourceId, Unavailable, UsageWindows, WindowId};

    use super::*;
    use crate::model::reset_time::{DAY, HOUR, MINUTE};
    use crate::reducer::reduce;
    use crate::state::AgentsState;
    use crate::testing::{
        claude, codex, connected, five_hour_limits, limits_answer,
        limits_loaded, moscow, now, with_profiles,
    };

    const HOUR_LATER: Duration = Duration::from_secs(HOUR);

    /// Agents mode with Claude Code connected and its limits read at
    /// `read_at` from `sources`.
    fn read_limits(
        state: &mut AgentsState,
        sources: Vec<(SourceId, Result<Limits, SourceError>)>,
        read_at: SystemTime,
    ) {
        let _ = reduce(
            state,
            limits_loaded(&claude(), limits_answer(sources), read_at),
        );
    }

    fn limits_of(state: &AgentsState) -> Option<LimitsViewModel> {
        LimitsViewModel::from_limits(
            state.profiles().shown()[0].limits(),
            now(),
            Some(&moscow()),
        )
    }

    /// When a window that resets at `resets_at` resets, as the Moscow user
    /// reads it at [`now`].
    fn resets_at(resets_at: SystemTime) -> Option<String> {
        Some(resets_text(now(), resets_at, Some(&moscow())))
    }

    /// The line under the credits: `left` credits left, and the pool
    /// renews at `renews_at`.
    fn left_and_renews(left: &str, renews_at: SystemTime) -> Option<String> {
        Some(i18n::filled(
            Key::TplAgentsCreditsLeftRenews,
            &[
                (
                    "left",
                    &i18n::filled(
                        Key::TplAgentsCreditsLeft,
                        &[("amount", left)],
                    ),
                ),
                ("renews", &resets_at(renews_at).unwrap_or_default()),
            ],
        ))
    }

    fn percent_used(percent: u64) -> String {
        i18n::with_count(Key::TplAgentsPercentUsed, percent)
    }

    #[test]
    fn an_answer_gives_one_bar_for_each_window_and_when_it_was_updated() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let five_hours = LimitWindow::new(
            WindowId::hours(5),
            41.6,
            Some(now() + HOUR_LATER),
        );
        let weekly = LimitWindow::new(WindowId::days(7), 103.4, None);
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
            vec![(
                crate::testing::named_source("Anthropic usage API"),
                Ok(limits),
            )],
            two_minutes_ago,
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(
            vm,
            LimitsViewModel {
                updated: Some(i18n::filled(
                    Key::TplAgentsUpdated,
                    &[("time", &i18n::with_count(Key::TplMinutesAgo, 2))]
                )),
                windows: vec![
                    UsageRowViewModel {
                        name: String::from("5 hours"),
                        used: percent_used(42),
                        percent: Some(42),
                        resets: resets_at(now() + HOUR_LATER),
                    },
                    UsageRowViewModel {
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
        let with_text = LimitWindow::new(WindowId::hours(5), 29.0, None)
            .with_reset_text("Resets 5pm (Europe/Berlin)");
        let with_both =
            LimitWindow::new(WindowId::days(7), 17.0, Some(now() + HOUR_LATER))
                .with_reset_text("Resets Nov 20, 10am (Europe/Berlin)");

        let text_bar = window_bar(&with_text, now(), Some(&moscow()));
        let both_bar = window_bar(&with_both, now(), Some(&moscow()));

        assert_eq!(
            text_bar.resets.as_deref(),
            Some("Resets 5pm (Europe/Berlin)")
        );
        assert_eq!(both_bar.resets, resets_at(now() + HOUR_LATER));
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
        read_limits(
            &mut state,
            vec![(
                crate::testing::named_source("Anthropic usage API"),
                Ok(limits),
            )],
            now(),
        );
        let with_limit = limits_of(&state).expect("the limits show");

        let spent_only =
            extra_usage_bar(&ExtraUsage::new(Some(4.2), None, None));

        assert_eq!(
            with_limit.windows,
            vec![UsageRowViewModel {
                name: i18n::t(Key::AgentsExtraUsage).to_string(),
                used: i18n::filled(
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
                limits_answer(vec![(
                    crate::testing::named_source("Latest Codex session"),
                    Ok(recorded),
                )]),
                now(),
            ),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(
            vm.recorded,
            Some(i18n::filled(
                Key::TplAgentsLimitsRecorded,
                &[("time", &i18n::with_count(Key::TplMinutesAgo, 30))]
            ))
        );
        assert!(!vm.is_stale, "the read itself answered");
    }

    #[test]
    fn the_last_good_answer_is_kept_greyed_with_its_time_after_a_failure() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let ten_minutes_ago = now() - Duration::from_secs(10 * MINUTE);
        read_limits(
            &mut state,
            vec![(
                crate::testing::named_source("Anthropic usage API"),
                Ok(five_hour_limits(42.0)),
            )],
            ten_minutes_ago,
        );
        read_limits(
            &mut state,
            vec![(
                crate::testing::named_source("Anthropic usage API"),
                Err(SourceError::Network(String::from("timed out"))),
            )],
            now(),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(vm.windows.len(), 1);
        assert_eq!(vm.windows[0].percent, Some(42));
        assert!(vm.is_stale);
        assert_eq!(vm.updated, Some(updated_text(now(), ten_minutes_ago)));
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsLimitsReadFailed,
                &format!("{}: network error: timed out", "Anthropic usage API")
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
                (
                    crate::testing::named_source("Anthropic usage API"),
                    Err(SourceError::NotLoggedIn),
                ),
                (
                    crate::testing::named_source("Claude Code /usage"),
                    Err(SourceError::NotAvailable(
                        Unavailable::ProgramNotFound("claude"),
                    )),
                ),
            ],
            now(),
        );

        let vm = limits_of(&state).expect("the reasons show");

        assert!(vm.windows.is_empty());
        assert!(!vm.is_stale);
        assert_eq!(vm.updated, None);
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsLimitsNotAvailable,
                &format!(
                    "{}: not logged in; {}: not available: claude is not found",
                    "Anthropic usage API", "Claude Code /usage"
                )
            ))
        );
        assert_eq!(vm.log_in_hint, Some(i18n::t(Key::AgentsLimitsLogInAgain)));
    }

    #[test]
    fn nothing_shows_before_a_read_or_for_an_agent_with_no_sources() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let _ = reduce(
            &mut state,
            limits_loaded(&codex(), crate::types::LimitsRead::NoSources, now()),
        );

        assert_eq!(limits_of(&state), None);
        assert_eq!(
            LimitsViewModel::from_limits(
                state.profiles().shown()[1].limits(),
                now(),
                Some(&moscow())
            ),
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
            Some(&moscow()),
        );
        let balance = credits_window(
            &Credits::new(Some(3760.0), None, None, None),
            now(),
            Some(&moscow()),
        );
        let unlimited =
            credits_window(&Credits::unlimited(), now(), Some(&moscow()));

        assert_eq!(
            pool,
            CreditsViewModel {
                value: i18n::filled(
                    Key::TplAgentsCreditsUsedOf,
                    &[("used", "1 240"), ("pool", "5 000")]
                ),
                percent: Some(25),
                line: left_and_renews("3 760", renews_at),
                note: None,
            }
        );
        assert_eq!(
            balance,
            CreditsViewModel {
                value: i18n::filled(
                    Key::TplAgentsCreditsLeft,
                    &[("amount", "3 760")]
                ),
                percent: None,
                line: None,
                note: None,
            },
            "the balance is the value and not repeated under it"
        );
        assert_eq!(unlimited.value, i18n::t(Key::AgentsCreditsUnlimited));
        assert_eq!(unlimited.note, None);
    }

    #[test]
    fn credits_with_no_number_are_available_with_a_note() {
        let renews_at = now() + Duration::from_secs(3 * DAY);
        let renewing = Credits::new(None, None, None, Some(renews_at));

        let available =
            credits_window(&Credits::available(), now(), Some(&moscow()));
        let renewing = credits_window(&renewing, now(), Some(&moscow()));

        assert_eq!(
            available,
            CreditsViewModel {
                value: i18n::t(Key::AgentsCreditsAvailable).to_string(),
                percent: None,
                line: None,
                note: Some(i18n::t(Key::AgentsCreditsNoBalance)),
            }
        );
        assert_eq!(
            renewing.line,
            Some(i18n::filled(
                Key::TplAgentsCreditsLeftRenews,
                &[
                    ("left", i18n::t(Key::AgentsCreditsAvailable)),
                    ("renews", &resets_at(renews_at).unwrap_or_default()),
                ]
            ))
        );
        assert_eq!(renewing.note, Some(i18n::t(Key::AgentsCreditsNoBalance)));
    }

    #[test]
    fn a_workspace_pool_has_its_used_part_a_bar_and_the_renewal() {
        let mut state = with_profiles(&[(codex(), connected())]);
        let renews_at = now() + Duration::from_secs(14 * DAY);
        let pool = Limits::new(
            Billing::Credits(Credits::new(
                Some(67_999.6),
                Some(12_000.4),
                Some(80_000.0),
                Some(renews_at),
            )),
            Some(String::from("Enterprise")),
        );
        let _ = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![(
                    crate::testing::named_source("ChatGPT usage API"),
                    Ok(pool),
                )]),
                now(),
            ),
        );

        let vm = limits_of(&state).expect("the credits show");

        assert_eq!(
            vm.credits,
            Some(CreditsViewModel {
                value: i18n::filled(
                    Key::TplAgentsCreditsUsedOf,
                    &[("used", "12 000"), ("pool", "80 000")]
                ),
                percent: Some(15),
                line: left_and_renews("68 000", renews_at),
                note: None,
            })
        );
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
                limits_answer(vec![(
                    crate::testing::named_source("codex app-server"),
                    Ok(limits),
                )]),
                now(),
            ),
        );

        let vm = limits_of(&state).expect("the credits show");

        assert!(vm.windows.is_empty());
        assert_eq!(
            vm.credits.map(|credits| credits.value),
            Some(i18n::filled(Key::TplAgentsCreditsLeft, &[("amount", "10")]))
        );
    }
}
