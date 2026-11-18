use std::time::SystemTime;

use otty_agents::{SourceError, Spend, TokenCount};
use otty_app_i18n::{self as i18n, Key};

use super::super::failure_text::source_error_text;
use super::super::spend::ProfileSpend;
use super::age::age_text;
use super::kept_answer::KeptAnswer;
use super::numbers::short_count;
use super::period::{DayBarViewModel, DayBarsViewModel, TotalViewModel};
use super::reset_time::date_text;

/// The "Tokens, last 30 days" block of a profile's dashboard: today, 7 and
/// 30 days, each split into in, cached and out, and a bar per day. The
/// last good count stays, greyed, when a later count fails (spec Decision
/// 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokensViewModel {
    /// Today, the last 7 days and the last 30 days; empty before a good
    /// count.
    pub(crate) totals: Vec<TotalViewModel>,
    /// The tokens of each of the last 30 days.
    pub(crate) days: Option<DayBarsViewModel>,
    /// Whether the numbers are an older count, drawn greyed, because the
    /// latest count failed.
    pub(crate) is_stale: bool,
    /// When the older count was made, while it is stale.
    pub(crate) last_read: Option<String>,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
}

impl TokensViewModel {
    /// The tokens of a profile at `now`; `None` while there is nothing to
    /// show: nothing was counted yet, or the agent can not count tokens.
    pub(crate) fn from_spend(
        spend: &ProfileSpend,
        now: SystemTime,
    ) -> Option<Self> {
        if !spend.has_counter() {
            return None;
        }

        match KeptAnswer::new(spend.last_good(), spend.failure()) {
            KeptAnswer::Nothing => None,
            KeptAnswer::OnlyFailure(error) => Some(Self {
                totals: Vec::new(),
                days: None,
                is_stale: false,
                last_read: None,
                failure: Some(count_failed_text(error)),
            }),
            KeptAnswer::Answer {
                value,
                read_at,
                failure,
            } => Some(Self {
                totals: totals(value),
                days: days(value),
                is_stale: failure.is_some(),
                last_read: failure.map(|_| {
                    i18n::agents_limits_last_read(&age_text(now, read_at))
                }),
                failure: failure.map(count_failed_text),
            }),
        }
    }
}

/// A number of tokens with its word, such as "2.1M tokens".
pub(crate) fn tokens_text(count: u64) -> String {
    i18n::agents_template(
        Key::TplAgentsTokensAmount,
        &[("amount", &short_count(count))],
    )
}

/// Why the latest count of the tokens failed, for the tokens and the
/// money of a dashboard: "Could not count the tokens: …".
pub(super) fn count_failed_text(error: &SourceError) -> String {
    i18n::with_error(Key::TplAgentsTokensFailed, &source_error_text(error))
}

/// The three totals of `spend`: the number and its split.
fn totals(spend: &Spend) -> Vec<TotalViewModel> {
    let total = |label: Key, count: TokenCount| TotalViewModel {
        label: i18n::t(label),
        value: short_count(count.total()),
        note: split_text(count),
    };

    vec![
        total(Key::AgentsToday, spend.today()),
        total(Key::AgentsLastSevenDays, spend.last_7_days()),
        total(Key::AgentsLastThirtyDays, spend.last_30_days()),
    ]
}

/// One bar for each of the last 30 days of `spend`; `None` when it has no
/// day.
fn days(spend: &Spend) -> Option<DayBarsViewModel> {
    let days = spend.last_30_days_by_day();
    let first_day = date_text(days.first()?.date());

    let bars = days
        .iter()
        .map(|day| {
            let tokens = day.tokens().total();
            DayBarViewModel {
                value: tokens,
                tip: i18n::agents_template(
                    Key::TplAgentsDayTokens,
                    &[
                        ("date", &date_text(day.date())),
                        ("tokens", &tokens_text(tokens)),
                    ],
                ),
            }
        })
        .collect();

    Some(DayBarsViewModel { bars, first_day })
}

/// `count` as one line: tokens in, cached and out, each written short.
fn split_text(count: TokenCount) -> String {
    i18n::agents_tokens(
        &short_count(count.input()),
        &short_count(count.cached()),
        &short_count(count.output()),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{DaySpend, ModelTokens, Unavailable};

    use super::*;
    use crate::testing::{at, now};

    fn spend(today: TokenCount, earlier: TokenCount) -> Spend {
        let day = |day: i8, tokens: TokenCount| {
            DaySpend::new(
                jiff::civil::date(2026, 11, day),
                vec![ModelTokens::new(Some("gpt-5"), tokens, 1)],
            )
        };

        Spend::new(vec![day(1, earlier), day(16, today)], Vec::new())
    }

    /// The tooltip of the bar of the day `date` with `tokens`.
    fn day_tip(date: &str, tokens: u64) -> String {
        i18n::agents_template(
            Key::TplAgentsDayTokens,
            &[("date", date), ("tokens", &tokens_text(tokens))],
        )
    }

    fn counted(spend: Spend) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(crate::types::SpendCount::Counted(spend), now());
        counted
    }

    #[test]
    fn today_7_and_30_days_show_the_total_and_in_cached_and_out() {
        let counted = counted(spend(
            TokenCount::new(12_345, 1_250_000, 456),
            TokenCount::new(1_000, 0, 44),
        ));

        let vm = TokensViewModel::from_spend(&counted, now())
            .expect("the tokens show");

        let total = |label: Key, value: &str, note: String| TotalViewModel {
            label: i18n::t(label),
            value: String::from(value),
            note,
        };
        assert_eq!(
            vm.totals,
            vec![
                total(
                    Key::AgentsToday,
                    "1.2M",
                    i18n::agents_tokens("12.3K", "1.2M", "456")
                ),
                total(
                    Key::AgentsLastSevenDays,
                    "1.2M",
                    i18n::agents_tokens("12.3K", "1.2M", "456")
                ),
                total(
                    Key::AgentsLastThirtyDays,
                    "1.2M",
                    i18n::agents_tokens("13.3K", "1.2M", "500")
                ),
            ]
        );
        assert!(!vm.is_stale);
        assert_eq!(vm.failure, None);
    }

    /// A day of November as the dashboard writes a date, such as "Nov 1".
    fn month_day(day: &str) -> String {
        i18n::agents_template(
            Key::TplAgentsMonthDay,
            &[("month", i18n::agents_month(11)), ("day", day)],
        )
    }

    /// Each date of the dashboard is written the same way, as the reset
    /// days are: "Nov 1", in the user's language.
    #[test]
    fn each_day_is_a_bar_with_its_date_and_tokens_in_the_tooltip() {
        let counted = counted(spend(
            TokenCount::new(2_900_000, 0, 0),
            TokenCount::new(500, 0, 0),
        ));

        let vm = TokensViewModel::from_spend(&counted, now())
            .expect("the tokens show");

        assert_eq!(
            vm.days,
            Some(DayBarsViewModel {
                bars: vec![
                    DayBarViewModel {
                        value: 500,
                        tip: day_tip(&month_day("1"), 500),
                    },
                    DayBarViewModel {
                        value: 2_900_000,
                        tip: day_tip(&month_day("16"), 2_900_000),
                    },
                ],
                first_day: month_day("1"),
            })
        );
    }

    #[test]
    fn a_failed_count_keeps_the_last_good_one_greyed_with_its_time() {
        let error = SourceError::NotAvailable(Unavailable::NoSessionFiles);
        let mut never_counted = ProfileSpend::default();
        never_counted
            .apply(crate::types::SpendCount::Failed(error.clone()), now());
        let ten_minutes_ago = now() - Duration::from_secs(600);
        let mut counted_before = ProfileSpend::default();
        counted_before.apply(
            crate::types::SpendCount::Counted(spend(
                TokenCount::new(1, 0, 0),
                TokenCount::default(),
            )),
            ten_minutes_ago,
        );
        counted_before
            .apply(crate::types::SpendCount::Failed(error.clone()), now());

        let never = TokensViewModel::from_spend(&never_counted, now())
            .expect("the failure shows");
        let before = TokensViewModel::from_spend(&counted_before, now())
            .expect("the last good count shows");

        let failure = i18n::with_error(
            Key::TplAgentsTokensFailed,
            "not available: no session files",
        );
        assert_eq!(
            never,
            TokensViewModel {
                totals: Vec::new(),
                days: None,
                is_stale: false,
                last_read: None,
                failure: Some(failure.clone()),
            }
        );
        assert_eq!(before.totals.len(), 3);
        assert!(before.is_stale);
        assert_eq!(
            before.last_read,
            Some(i18n::agents_limits_last_read(&i18n::with_count(
                Key::TplAgentsAgeMinutes,
                10
            )))
        );
        assert_eq!(before.failure, Some(failure));
    }

    #[test]
    fn nothing_shows_before_a_count_or_without_a_counter() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(crate::types::SpendCount::NoCounter, at(10));

        assert_eq!(
            TokensViewModel::from_spend(&ProfileSpend::default(), now()),
            None
        );
        assert_eq!(TokensViewModel::from_spend(&no_counter, now()), None);
    }
}
