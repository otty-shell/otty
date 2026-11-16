use std::time::SystemTime;

use otty_agents::{Bill, Dollars, MoneyEstimate, Spend};
use otty_app_i18n::{self as i18n, Key};

use super::super::age::age_text;
use super::super::spend::ProfileSpend;
use super::numbers::dollars_text;
use super::period::{
    DayBarViewModel, DayBarsViewModel, TotalViewModel, date_text,
};
use super::tokens::tokens_text;

/// The Spending block of an API key profile's dashboard (spec Decision 9):
/// the money of today, 7 days and this month, a bar per day, the budget
/// bar, and where the money comes from: the bill of the organization, or
/// an estimate at the prices of a date with the requests with no price.
/// The last good estimate stays, greyed, when a later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// That the money is from the bill of the whole organization, or an
    /// estimate with the date of the prices.
    pub(crate) note: Option<String>,
    /// Today, the last 7 days and this month, each with its tokens; empty
    /// before a good count.
    pub(crate) totals: Vec<TotalViewModel>,
    /// The money of each of the last 30 days.
    pub(crate) days: Option<DayBarsViewModel>,
    /// The money of this month against the user's monthly budget, when one
    /// is set.
    pub(crate) budget: Option<BudgetViewModel>,
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
    /// The money of an API key profile from the `bill` of its
    /// organization, against its monthly `budget`.
    pub(crate) fn from_bill(bill: &Bill, budget: Option<Dollars>) -> Self {
        let total = |label: Key, cost: Dollars| TotalViewModel {
            label: i18n::t(label),
            value: dollars_text(cost),
            note: String::new(),
        };

        Self {
            note: Some(i18n::t(Key::AgentsMoneyFromBill).to_string()),
            totals: vec![
                total(Key::AgentsToday, bill.today()),
                total(Key::AgentsLastSevenDays, bill.last_7_days()),
                total(Key::AgentsThisMonth, bill.this_month()),
            ],
            days: bill_days(bill),
            budget: budget
                .map(|budget| BudgetViewModel::new(bill.this_month(), budget)),
            unpriced: None,
            is_stale: false,
            last_read: None,
            failure: None,
        }
    }

    /// The money of an API key profile at `now` from its `money` estimate
    /// and the counts of `spend` it came from, against its monthly
    /// `budget`; `None` while there is nothing to show.
    pub(crate) fn from_money(
        money: Option<&MoneyEstimate>,
        spend: &ProfileSpend,
        budget: Option<Dollars>,
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
                budget: None,
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
            budget: budget.map(|budget| {
                BudgetViewModel::new(money.this_month().cost(), budget)
            }),
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

/// The money of this month against the user's monthly budget (spec
/// Decision 9): only drawn as a bar, nothing stops or warns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BudgetViewModel {
    /// The money spent of the budget, such as "$184.75 of $300.00".
    pub(crate) spent_of: String,
    /// How much of the budget is spent, from 0 to 100; a spend over the
    /// budget fills the bar.
    pub(crate) percent: u8,
}

impl BudgetViewModel {
    /// `spent` of the month against `budget`. A budget of nothing is full
    /// as soon as anything is spent.
    fn new(spent: Dollars, budget: Dollars) -> Self {
        let percent = if budget.micro_dollars() == 0 {
            if spent.micro_dollars() == 0 {
                0.0
            } else {
                100.0
            }
        } else {
            spent.micro_dollars() as f64 * 100.0 / budget.micro_dollars() as f64
        };

        Self {
            spent_of: i18n::agents_template(
                Key::TplAgentsAmountOf,
                &[
                    ("amount", &dollars_text(spent)),
                    ("limit", &dollars_text(budget)),
                ],
            ),
            percent: percent.clamp(0.0, 100.0).round() as u8,
        }
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

/// The bars of the money of each day of the bill.
fn bill_days(bill: &Bill) -> Option<DayBarsViewModel> {
    let days = bill.last_30_days_by_day();
    let first_day = date_text(days.first()?.date());

    let bars = days
        .iter()
        .map(|day| DayBarViewModel {
            value: day.cost().micro_dollars(),
            tip: format!(
                "{}: {}",
                date_text(day.date()),
                dollars_text(day.cost())
            ),
        })
        .collect();

    Some(DayBarsViewModel { bars, first_day })
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
    use crate::testing::{bill_of, dollars, now};

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

        let vm = MoneyViewModel::from_money(
            Some(&money),
            &counted(now()),
            None,
            now(),
        )
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

        let vm = MoneyViewModel::from_money(
            Some(&money),
            &counted(now()),
            None,
            now(),
        )
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

        let vm = MoneyViewModel::from_money(Some(&money), &spend, None, now())
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
    fn the_bill_says_it_is_the_bill_of_the_whole_organization() {
        let bill = bill_of(12, 3);

        let vm = MoneyViewModel::from_bill(&bill, None);

        assert_eq!(
            vm.note,
            Some(i18n::t(Key::AgentsMoneyFromBill).to_string())
        );
        let values: Vec<_> =
            vm.totals.iter().map(|total| total.value.as_str()).collect();
        assert_eq!(values, ["$12.00", "$15.00", "$15.00"]);
        assert_eq!(vm.unpriced, None);
        assert_eq!(vm.budget, None);
        let tips: Vec<_> = vm
            .days
            .expect("a bar per day")
            .bars
            .into_iter()
            .map(|bar| bar.tip)
            .collect();
        assert_eq!(tips, ["2026-11-11: $3.00", "2026-11-16: $12.00"]);
    }

    #[test]
    fn the_budget_bar_is_the_money_of_this_month_of_the_budget() {
        let money = estimate();
        let budget = Some(dollars(300));

        let from_estimate = MoneyViewModel::from_money(
            Some(&money),
            &counted(now()),
            budget,
            now(),
        )
        .and_then(|vm| vm.budget);
        let from_bill =
            MoneyViewModel::from_bill(&bill_of(12, 3), budget).budget;

        assert_eq!(
            from_estimate,
            Some(BudgetViewModel {
                spent_of: i18n::agents_template(
                    Key::TplAgentsAmountOf,
                    &[("amount", "$3.75"), ("limit", "$300.00")]
                ),
                percent: 1,
            })
        );
        assert_eq!(from_bill.map(|budget| budget.percent), Some(5));
    }

    #[test]
    fn spending_over_the_budget_only_fills_the_bar() {
        let vm =
            MoneyViewModel::from_bill(&bill_of(400, 0), Some(dollars(300)));

        let budget = vm.budget.expect("the budget shows");

        assert_eq!(budget.percent, 100);
        assert_eq!(
            budget.spent_of,
            i18n::agents_template(
                Key::TplAgentsAmountOf,
                &[("amount", "$400.00"), ("limit", "$300.00")]
            )
        );
        assert_eq!(vm.failure, None, "nothing warns beyond the bar");
    }

    #[test]
    fn nothing_shows_before_a_count_or_without_a_counter() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(None, now());

        assert_eq!(
            MoneyViewModel::from_money(
                None,
                &ProfileSpend::default(),
                None,
                now()
            ),
            None
        );
        assert_eq!(
            MoneyViewModel::from_money(None, &no_counter, None, now()),
            None
        );
    }
}
