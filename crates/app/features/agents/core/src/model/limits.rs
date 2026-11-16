use std::time::SystemTime;

use otty_agents::{Billing, Credits, LimitWindow, SourceError};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::limits::ProfileLimits;
use super::failure_reasons;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// The limits of one profile as the left panel and the details draw them
/// (spec Decisions 5 and 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LimitsViewModel {
    /// One bar for each usage window, in the order of the answer.
    pub(crate) bars: Vec<LimitBarViewModel>,
    /// The credits left of an account paid for by credits, when known.
    pub(crate) credits: Option<String>,
    /// The name of the source that answered.
    pub(crate) source: Option<String>,
    /// Why the better sources failed before the one that answered.
    pub(crate) earlier_failures: Option<String>,
    /// Whether the bars are an older answer, drawn greyed out, because the
    /// latest read failed.
    pub(crate) is_stale: bool,
    /// When the older answer was read, while it is stale.
    pub(crate) last_read: Option<String>,
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
        let is_login_reason = failures.is_some_and(|failures| {
            failures
                .iter()
                .any(|failure| *failure.error() == SourceError::NotLoggedIn)
        });
        let log_in_hint =
            is_login_reason.then(|| i18n::t(Key::AgentsLimitsLogInAgain));

        let Some((answer, read_at)) = limits.last_good() else {
            let failures = failures?;
            return Some(Self {
                bars: Vec::new(),
                credits: None,
                source: None,
                earlier_failures: None,
                is_stale: false,
                last_read: None,
                recorded: None,
                failure: Some(i18n::with_error(
                    Key::TplAgentsLimitsNotAvailable,
                    &failure_reasons(failures),
                )),
                log_in_hint,
            });
        };

        let value = answer.value();
        let is_stale = failures.is_some();
        let earlier = answer.earlier_failures();

        let (bars, credits) = match value.billing() {
            Billing::UsageWindows(usage) => {
                let bars = usage
                    .windows()
                    .iter()
                    .map(|window| LimitBarViewModel::from_window(window, now))
                    .collect();
                (bars, None)
            },
            Billing::Credits(credits) => (Vec::new(), credits_text(credits)),
            _ => (Vec::new(), None),
        };

        Some(Self {
            bars,
            credits,
            source: Some(answer.source().to_string()),
            earlier_failures: (!earlier.is_empty()).then(|| {
                i18n::with_error(
                    Key::TplAgentsLimitsEarlierFailed,
                    &failure_reasons(earlier),
                )
            }),
            is_stale,
            last_read: is_stale.then(|| {
                i18n::agents_limits_last_read(&age_text(now, read_at))
            }),
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

/// One window as a bar: its name, how much is used and when it resets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LimitBarViewModel {
    /// The name of the window, such as "5 hours".
    pub(crate) name: String,
    /// How much is used, from 0 to 100.
    pub(crate) percent: u8,
    /// When the window starts again, such as "resets in 3 h".
    pub(crate) resets: Option<String>,
}

impl LimitBarViewModel {
    /// The bar of `window` at `now`. The percent is rounded and kept
    /// between 0 and 100.
    fn from_window(window: &LimitWindow, now: SystemTime) -> Self {
        let percent = window.percent_used().clamp(0.0, 100.0).round() as u8;

        Self {
            name: window.name().to_string(),
            percent,
            resets: window
                .resets_at()
                .map(|resets_at| resets_text(now, resets_at)),
        }
    }
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

/// The credits: "Unlimited", or the balance with at most two decimals;
/// `None` when the source gave neither.
fn credits_text(credits: &Credits) -> Option<String> {
    if credits.is_unlimited() {
        return Some(i18n::t(Key::AgentsCreditsUnlimited).to_string());
    }

    let left = credits.left()?;
    let rounded = (left * 100.0).round() / 100.0;

    Some(format!("{rounded}"))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{Billing, Limits, SourceError, UsageWindows};

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

    #[test]
    fn an_answer_gives_one_bar_for_each_window() {
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
        read_limits(&mut state, vec![("usage request", Ok(limits))], now());

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(
            vm,
            LimitsViewModel {
                bars: vec![
                    LimitBarViewModel {
                        name: String::from("5 hours"),
                        percent: 42,
                        resets: Some(i18n::with_count(
                            Key::TplAgentsResetsInHours,
                            1
                        )),
                    },
                    LimitBarViewModel {
                        name: String::from("7 days"),
                        percent: 100,
                        resets: None,
                    },
                ],
                credits: None,
                source: Some(String::from("usage request")),
                earlier_failures: None,
                is_stale: false,
                last_read: None,
                recorded: None,
                failure: None,
                log_in_hint: None,
            }
        );
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
    fn numbers_told_now_say_nothing_of_their_age() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read_limits(
            &mut state,
            vec![("usage request", Ok(five_hour_limits(42.0)))],
            now(),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(vm.recorded, None);
    }

    #[test]
    fn an_answer_of_a_later_source_says_why_the_earlier_ones_failed() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read_limits(
            &mut state,
            vec![
                ("usage request", Err(SourceError::NotLoggedIn)),
                ("session files", Ok(five_hour_limits(10.0))),
            ],
            now(),
        );

        let vm = limits_of(&state).expect("the limits show");

        assert_eq!(vm.source.as_deref(), Some("session files"));
        assert_eq!(
            vm.earlier_failures,
            Some(i18n::with_error(
                Key::TplAgentsLimitsEarlierFailed,
                "usage request: not logged in"
            ))
        );
        assert_eq!(vm.log_in_hint, None, "a source answered");
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

        assert_eq!(vm.bars.len(), 1);
        assert_eq!(vm.bars[0].percent, 42);
        assert!(vm.is_stale);
        assert_eq!(
            vm.last_read,
            Some(i18n::agents_limits_last_read(&i18n::with_count(
                Key::TplAgentsAgeMinutes,
                10
            )))
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

        assert!(vm.bars.is_empty());
        assert!(!vm.is_stale);
        assert_eq!(vm.source, None);
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

    /// Limits paid for by `credits`, read from the usage request.
    fn credits_of(credits: Credits) -> Option<String> {
        let mut state = with_profiles(&[(claude(), connected())]);
        let limits = Limits::new(Billing::Credits(credits), None);
        read_limits(&mut state, vec![("usage request", Ok(limits))], now());

        limits_of(&state).expect("the limits show").credits
    }

    #[test]
    fn credits_read_as_unlimited_or_the_rounded_balance_with_no_bars() {
        let unlimited = credits_of(Credits::unlimited());
        let balance = credits_of(Credits::new(Some(12.346), None, None, None));
        let whole = credits_of(Credits::new(Some(3.0), None, None, None));
        let unknown = credits_of(Credits::new(None, None, None, None));

        assert_eq!(
            unlimited.as_deref(),
            Some(i18n::t(Key::AgentsCreditsUnlimited))
        );
        assert_eq!(balance.as_deref(), Some("12.35"));
        assert_eq!(whole.as_deref(), Some("3"));
        assert_eq!(unknown, None);
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
