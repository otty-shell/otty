use std::time::SystemTime;

use otty_agents::{Dollars, MoneyEstimate, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::spend::ProfileSpend;
use super::numbers::dollars_text;
use super::period::{
    DayBarViewModel, DayBarsViewModel, TotalViewModel, date_text,
};
use super::tokens::tokens_text;

/// The Spending block of an API key profile's dashboard (spec Decision 9):
/// the money of today, 7 days and this month, a bar per day, that it is an
/// estimate at the prices of a date, and the requests with no price. The
/// last good estimate stays, greyed, when a later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// That the money is an estimate, with the date of the prices.
    pub(crate) note: Option<String>,
    /// Today, the last 7 days and this month, each with its tokens; empty
    /// before a good count.
    pub(crate) totals: Vec<TotalViewModel>,
    /// The money of each of the last 30 days.
    pub(crate) days: Option<DayBarsViewModel>,
    /// How many requests of this month have no price, when any.
    pub(crate) unpriced: Option<String>,
    /// Whether the money is an older estimate, drawn greyed, because the
    /// latest count failed.
    pub(crate) is_stale: bool,
    /// When the older count was made, while it is stale.
    pub(crate) last_read: Option<String>,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
}

impl MoneyViewModel {
    /// The money of an API key profile at `now` from its `money` estimate
    /// and the counts of `spend` it came from; `None` while there is
    /// nothing to show.
    pub(crate) fn from_money(
        money: Option<&MoneyEstimate>,
        spend: &ProfileSpend,
        now: SystemTime,
    ) -> Option<Self> {
        if !spend.has_counter() {
            return None;
        }

        let failure = spend.failure().map(|error| {
            i18n::with_error(Key::TplAgentsTokensFailed, &error.to_string())
        });
        let counted = spend.last_good().zip(money);
        if counted.is_none() && failure.is_none() {
            return None;
        }

        let is_stale = counted.is_some() && failure.is_some();
        let Some(((tokens, counted_at), money)) = counted else {
            return Some(Self {
                note: None,
                totals: Vec::new(),
                days: None,
                unpriced: None,
                is_stale,
                last_read: None,
                failure,
            });
        };

        let unpriced = money.this_month().unpriced_requests();
        Some(Self {
            note: Some(i18n::agents_template(
                Key::TplAgentsMoneyEstimate,
                &[("date", &date_text(money.prices_taken_on()))],
            )),
            totals: totals(money, tokens),
            days: days(money),
            unpriced: (unpriced > 0).then(|| {
                i18n::agents_template(
                    Key::TplAgentsMoneyUnpriced,
                    &[("count", &format!("{unpriced}"))],
                )
            }),
            is_stale,
            last_read: is_stale.then(|| {
                i18n::agents_limits_last_read(&age_text(now, counted_at))
            }),
            failure,
        })
    }
}

/// The three totals of the money, each with the tokens it is made of.
fn totals(money: &MoneyEstimate, spend: &Spend) -> Vec<TotalViewModel> {
    let month_tokens: u64 = spend
        .this_month_by_model()
        .iter()
        .map(|model| model.tokens().total())
        .sum();
    let total = |label: Key, cost: Dollars, tokens: u64| TotalViewModel {
        label: i18n::t(label),
        value: dollars_text(cost),
        note: tokens_text(tokens),
    };

    vec![
        total(
            Key::AgentsToday,
            money.today().cost(),
            spend.today().total(),
        ),
        total(
            Key::AgentsLastSevenDays,
            money.last_7_days().cost(),
            spend.last_7_days().total(),
        ),
        total(
            Key::AgentsThisMonth,
            money.this_month().cost(),
            month_tokens,
        ),
    ]
}

/// The bars of the money of each day.
fn days(money: &MoneyEstimate) -> Option<DayBarsViewModel> {
    let days = money.last_30_days_by_day();
    let first_day = date_text(days.first()?.date());

    let bars = days
        .iter()
        .map(|day| {
            let cost = day.total().cost();
            DayBarViewModel {
                value: cost.micro_dollars(),
                tip: format!(
                    "{}: {}",
                    date_text(day.date()),
                    dollars_text(cost)
                ),
            }
        })
        .collect();

    Some(DayBarsViewModel { bars, first_day })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{
        CalendarDate, DaySpend, ModelTokens, SourceError, TokenCount,
    };

    use super::*;
    use crate::testing::now;

    /// One million input tokens of `gpt-5` (US$1.25 at the bundled prices)
    /// on 2026-11-02 and two millions today, 2026-11-16, and 3 requests of
    /// a model with no price today.
    fn spend() -> Spend {
        let gpt5 = |millions: u64| {
            ModelTokens::new(
                Some("gpt-5"),
                TokenCount::new(millions * 1_000_000, 0, 0),
                1,
            )
        };
        let unknown = ModelTokens::new(
            Some("no-such-model"),
            TokenCount::new(10, 0, 0),
            3,
        );

        Spend::new(
            vec![
                DaySpend::new(CalendarDate::new(2026, 11, 2), vec![gpt5(1)]),
                DaySpend::new(
                    CalendarDate::new(2026, 11, 16),
                    vec![gpt5(2), unknown],
                ),
            ],
            Vec::new(),
        )
    }

    fn counted(at: SystemTime) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend())), at);
        counted
    }

    fn estimate() -> MoneyEstimate {
        MoneyEstimate::new(&spend(), &otty_agents::bundled_prices())
    }

    #[test]
    fn today_7_days_and_this_month_show_the_money_and_its_tokens() {
        let money = estimate();

        let vm =
            MoneyViewModel::from_money(Some(&money), &counted(now()), now())
                .expect("the money shows");

        let total = |label: Key, value: &str, tokens: u64| TotalViewModel {
            label: i18n::t(label),
            value: String::from(value),
            note: tokens_text(tokens),
        };
        assert_eq!(
            vm.totals,
            vec![
                total(Key::AgentsToday, "$2.50", 2_000_010),
                total(Key::AgentsLastSevenDays, "$2.50", 2_000_010),
                total(Key::AgentsThisMonth, "$3.75", 3_000_010),
            ]
        );
        assert_eq!(
            vm.note,
            Some(i18n::agents_template(
                Key::TplAgentsMoneyEstimate,
                &[("date", "2026-11-16")]
            ))
        );
        assert_eq!(
            vm.unpriced,
            Some(i18n::agents_template(
                Key::TplAgentsMoneyUnpriced,
                &[("count", "3")]
            ))
        );
        assert!(!vm.is_stale);
    }

    #[test]
    fn each_day_is_a_bar_of_its_money() {
        let money = estimate();

        let vm =
            MoneyViewModel::from_money(Some(&money), &counted(now()), now())
                .expect("the money shows");

        assert_eq!(
            vm.days,
            Some(DayBarsViewModel {
                bars: vec![
                    DayBarViewModel {
                        value: 1_250_000,
                        tip: String::from("2026-11-02: $1.25"),
                    },
                    DayBarViewModel {
                        value: 2_500_000,
                        tip: String::from("2026-11-16: $2.50"),
                    },
                ],
                first_day: String::from("2026-11-02"),
            })
        );
    }

    #[test]
    fn a_failed_count_keeps_the_last_estimate_greyed_with_its_time() {
        let money = estimate();
        let ten_minutes_ago = now() - Duration::from_secs(600);
        let mut spend = counted(ten_minutes_ago);
        let error = SourceError::NotAvailable(String::from("no folder"));
        spend.apply(Some(Err(error)), now());

        let vm = MoneyViewModel::from_money(Some(&money), &spend, now())
            .expect("the last estimate shows");

        assert!(vm.is_stale);
        assert_eq!(vm.totals.len(), 3);
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
                Key::TplAgentsTokensFailed,
                "not available: no folder"
            ))
        );
    }

    #[test]
    fn nothing_shows_before_a_count_or_without_a_counter() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(None, now());

        assert_eq!(
            MoneyViewModel::from_money(None, &ProfileSpend::default(), now()),
            None
        );
        assert_eq!(MoneyViewModel::from_money(None, &no_counter, now()), None);
    }
}
