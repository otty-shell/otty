use std::time::SystemTime;

use otty_agents::{Spend, TokenCount};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::spend::ProfileSpend;
use super::numbers::short_count;
use super::period::{
    DayBarViewModel, DayBarsViewModel, TotalViewModel, date_text,
};

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

        let failure = spend.failure().map(|error| {
            i18n::with_error(Key::TplAgentsTokensFailed, &error.to_string())
        });
        let last_good = spend.last_good();
        if last_good.is_none() && failure.is_none() {
            return None;
        }

        let is_stale = last_good.is_some() && failure.is_some();
        Some(Self {
            totals: last_good.map_or_else(Vec::new, |(spend, _)| totals(spend)),
            days: last_good.and_then(|(spend, _)| days(spend)),
            is_stale,
            last_read: last_good.filter(|_| is_stale).map(|(_, counted_at)| {
                i18n::agents_limits_last_read(&age_text(now, counted_at))
            }),
            failure,
        })
    }
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
                tip: format!(
                    "{}: {}",
                    date_text(day.date()),
                    tokens_text(tokens)
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

/// A number of tokens with its word, such as "2.1M tokens".
pub(crate) fn tokens_text(count: u64) -> String {
    i18n::agents_template(
        Key::TplAgentsTokensAmount,
        &[("amount", &short_count(count))],
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{
        CalendarDate, DaySpend, ModelTokens, SourceError, Spend,
    };

    use super::*;
    use crate::testing::{at, now};

    fn spend(today: TokenCount, earlier: TokenCount) -> Spend {
        let day = |day: u8, tokens: TokenCount| {
            DaySpend::new(
                CalendarDate::new(2026, 11, day),
                vec![ModelTokens::new(Some("gpt-5"), tokens, 1)],
            )
        };

        Spend::new(vec![day(1, earlier), day(16, today)], Vec::new())
    }

    fn counted(spend: Spend) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend)), now());
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
                        tip: format!("2026-11-01: {}", tokens_text(500)),
                    },
                    DayBarViewModel {
                        value: 2_900_000,
                        tip: format!("2026-11-16: {}", tokens_text(2_900_000)),
                    },
                ],
                first_day: String::from("2026-11-01"),
            })
        );
    }

    #[test]
    fn a_failed_count_keeps_the_last_good_one_greyed_with_its_time() {
        let error = SourceError::NotAvailable(String::from("no folder"));
        let mut never_counted = ProfileSpend::default();
        never_counted.apply(Some(Err(error.clone())), now());
        let ten_minutes_ago = now() - Duration::from_secs(600);
        let mut counted_before = ProfileSpend::default();
        counted_before.apply(
            Some(Ok(spend(TokenCount::new(1, 0, 0), TokenCount::default()))),
            ten_minutes_ago,
        );
        counted_before.apply(Some(Err(error.clone())), now());

        let never = TokensViewModel::from_spend(&never_counted, now())
            .expect("the failure shows");
        let before = TokensViewModel::from_spend(&counted_before, now())
            .expect("the last good count shows");

        let failure = i18n::with_error(
            Key::TplAgentsTokensFailed,
            "not available: no folder",
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
        no_counter.apply(None, at(10));

        assert_eq!(
            TokensViewModel::from_spend(&ProfileSpend::default(), now()),
            None
        );
        assert_eq!(TokensViewModel::from_spend(&no_counter, now()), None);
    }
}
