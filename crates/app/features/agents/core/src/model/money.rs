use std::time::SystemTime;

use otty_agents::{Bill, Dollars, MoneyEstimate};
use otty_app_i18n::{self as i18n, Key};

use super::super::prices::ListPrices;
use super::super::spend::ProfileSpend;
use super::limits::{UsageRowViewModel, updated_text};
use super::numbers::dollars_text;

/// What stands for an amount of money that can not be known without
/// prices.
const NO_AMOUNT: &str = "—";

/// The Usage block of an API key profile's dashboard (spec Decisions 9, 10
/// and 12): when the money was read, then the money of today, 7 days and
/// this month as rows like the usage windows of a subscription, the budget
/// row with its bar when one is set, and the requests with no price. What
/// the money is, the bill or an estimate, is said only by the sources. The
/// last good estimate stays, greyed, when a later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// When the money was read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// Today, the last 7 days, this month and the budget; empty before a
    /// good count.
    pub(crate) rows: Vec<UsageRowViewModel>,
    /// The muted line under the rows: how many requests of this month
    /// have no price, or that no prices could be read.
    pub(crate) note: Option<String>,
    /// Whether the money is an older estimate, drawn greyed, because the
    /// latest count failed.
    pub(crate) is_stale: bool,
    /// Why the latest count failed.
    pub(crate) failure: Option<String>,
}

impl MoneyViewModel {
    /// The money of an API key profile at `now` from the `bill` of its
    /// organization read at `read_at`, against its monthly `budget`.
    pub(crate) fn from_bill(
        bill: &Bill,
        read_at: SystemTime,
        budget: Option<Dollars>,
        now: SystemTime,
    ) -> Self {
        Self {
            updated: Some(updated_text(now, read_at)),
            rows: money_rows(
                [bill.today(), bill.last_7_days(), bill.this_month()],
                budget,
            ),
            note: None,
            is_stale: false,
            failure: None,
        }
    }

    /// The money of an API key profile at `now` from its `money` estimate
    /// and the count of `spend` it came from, against its monthly
    /// `budget`; `None` while there is nothing to show. While there are no
    /// `prices`, each amount is a dash, and a line says when no source gave
    /// them (spec Decision 16).
    pub(crate) fn from_money(
        money: Option<&MoneyEstimate>,
        spend: &ProfileSpend,
        prices: &ListPrices,
        budget: Option<Dollars>,
        now: SystemTime,
    ) -> Option<Self> {
        if !spend.has_counter() {
            return None;
        }

        let failure = spend.failure().map(|error| {
            i18n::with_error(Key::TplAgentsTokensFailed, &error.to_string())
        });
        let Some((_, counted_at)) = spend.last_good() else {
            return failure.map(|failure| Self {
                updated: None,
                rows: Vec::new(),
                note: None,
                is_stale: false,
                failure: Some(failure),
            });
        };
        let updated = Some(updated_text(now, counted_at));
        let is_stale = failure.is_some();

        let Some(money) = money else {
            let no_prices = prices
                .has_failed()
                .then(|| i18n::t(Key::AgentsNoPrices).to_string());
            return Some(Self {
                updated,
                rows: unknown_money_rows(),
                note: no_prices,
                is_stale,
                failure,
            });
        };

        let unpriced = money.this_month().unpriced_requests();
        let spent = [
            money.today().cost(),
            money.last_7_days().cost(),
            money.this_month().cost(),
        ];
        Some(Self {
            updated,
            rows: money_rows(spent, budget),
            note: (unpriced > 0).then(|| {
                i18n::agents_template(
                    Key::TplAgentsMoneyUnpriced,
                    &[("count", &format!("{unpriced}"))],
                )
            }),
            is_stale,
            failure,
        })
    }
}

/// The rows of the money `spent` today, in the last 7 days and this month,
/// then the row of the monthly `budget` when one is set.
fn money_rows(
    spent: [Dollars; 3],
    budget: Option<Dollars>,
) -> Vec<UsageRowViewModel> {
    let [today, last_7_days, this_month] = spent;
    let row = |name: Key, cost: Dollars| UsageRowViewModel {
        name: i18n::t(name).to_string(),
        used: dollars_text(cost),
        percent: None,
        resets: None,
    };

    let mut rows = vec![
        row(Key::AgentsToday, today),
        row(Key::AgentsLastSevenDays, last_7_days),
        row(Key::AgentsThisMonth, this_month),
    ];
    rows.extend(budget.map(|budget| budget_row(this_month, budget)));

    rows
}

/// The rows of today, the last 7 days and this month while there are no
/// prices: a dash in place of each amount, as "$0.00" would read as
/// nothing spent, and no budget bar (spec Decision 16).
fn unknown_money_rows() -> Vec<UsageRowViewModel> {
    [
        Key::AgentsToday,
        Key::AgentsLastSevenDays,
        Key::AgentsThisMonth,
    ]
    .into_iter()
    .map(|name| UsageRowViewModel {
        name: i18n::t(name).to_string(),
        used: String::from(NO_AMOUNT),
        percent: None,
        resets: None,
    })
    .collect()
}

/// The money `spent` this month against the monthly `budget` (spec
/// Decision 9): only drawn as a bar, nothing stops or warns. A spend over
/// the budget fills the bar; a budget of nothing is full as soon as
/// anything is spent.
fn budget_row(spent: Dollars, budget: Dollars) -> UsageRowViewModel {
    let percent = if budget.micro_dollars() == 0 {
        if spent.micro_dollars() == 0 {
            0.0
        } else {
            100.0
        }
    } else {
        spent.micro_dollars() as f64 * 100.0 / budget.micro_dollars() as f64
    };

    UsageRowViewModel {
        name: i18n::t(Key::AgentsBudgetTitle).to_string(),
        used: i18n::agents_template(
            Key::TplAgentsAmountOf,
            &[
                ("amount", &dollars_text(spent)),
                ("limit", &dollars_text(budget)),
            ],
        ),
        percent: Some(percent.clamp(0.0, 100.0).round() as u8),
        resets: None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{
        CalendarDate, DaySpend, ModelTokens, SourceError, SourceId, Spend,
        TokenCount,
    };

    use super::*;
    use crate::testing::{bill_of, dollars, now, price_table, prices_answer};

    /// One million input tokens of `gpt-5` (US$1.25 at the made-up prices)
    /// on 2026-11-02 and two millions today, 2026-11-16, and `unpriced`
    /// requests of a model with no price today.
    fn spend_with(unpriced: u64) -> Spend {
        let gpt5 = |millions: u64| {
            ModelTokens::new(
                Some("gpt-5"),
                TokenCount::new(millions * 1_000_000, 0, 0),
                1,
            )
        };
        let mut today = vec![gpt5(2)];
        if unpriced > 0 {
            today.push(ModelTokens::new(
                Some("no-such-model"),
                TokenCount::new(10, 0, 0),
                unpriced,
            ));
        }

        Spend::new(
            vec![
                DaySpend::new(CalendarDate::new(2026, 11, 2), vec![gpt5(1)]),
                DaySpend::new(CalendarDate::new(2026, 11, 16), today),
            ],
            Vec::new(),
        )
    }

    fn counted(spend: Spend, at: SystemTime) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(Some(Ok(spend)), at);
        counted
    }

    fn estimate(spend: &Spend) -> MoneyEstimate {
        MoneyEstimate::new(spend, &price_table())
    }

    /// A read of the prices in which no source gave any.
    fn failed_prices() -> ListPrices {
        let failures = [
            (
                SourceId::LiteLlmPrices,
                SourceError::Network(String::from("down")),
            ),
            (
                SourceId::KeptPrices,
                SourceError::NotAvailable(String::from("no kept price file")),
            ),
        ];

        let mut prices = ListPrices::default();
        prices.apply(prices_answer(&failures, None), now());
        prices
    }

    /// Prices LiteLLM gave just now.
    fn good_prices() -> ListPrices {
        let fetched = otty_agents::FetchedPrices::new(price_table(), now());

        let mut prices = ListPrices::default();
        prices.apply(prices_answer(&[], Some(fetched)), now());
        prices
    }

    /// The money of the estimate with `unpriced` requests, against
    /// `budget`.
    fn from_estimate(unpriced: u64, budget: Option<Dollars>) -> MoneyViewModel {
        let spend = spend_with(unpriced);
        let money = estimate(&spend);

        MoneyViewModel::from_money(
            Some(&money),
            &counted(spend, now()),
            &good_prices(),
            budget,
            now(),
        )
        .expect("the money shows")
    }

    /// A row of money with no bar, such as "Today  $2.50".
    fn row(name: Key, used: &str) -> UsageRowViewModel {
        UsageRowViewModel {
            name: i18n::t(name).to_string(),
            used: String::from(used),
            percent: None,
            resets: None,
        }
    }

    /// The row of the budget: what is spent of it and its bar.
    fn budget_row(spent: &str, budget: &str, percent: u8) -> UsageRowViewModel {
        UsageRowViewModel {
            name: i18n::t(Key::AgentsBudgetTitle).to_string(),
            used: i18n::agents_template(
                Key::TplAgentsAmountOf,
                &[("amount", spent), ("limit", budget)],
            ),
            percent: Some(percent),
            resets: None,
        }
    }

    #[test]
    fn with_no_budget_the_money_is_three_rows_with_no_bar() {
        let vm = from_estimate(0, None);

        assert_eq!(
            vm.rows,
            vec![
                row(Key::AgentsToday, "$2.50"),
                row(Key::AgentsLastSevenDays, "$2.50"),
                row(Key::AgentsThisMonth, "$3.75"),
            ]
        );
        assert_eq!(vm.updated, Some(updated_text(now(), now())));
        assert_eq!(vm.note, None);
        assert!(!vm.is_stale);
    }

    #[test]
    fn with_a_budget_the_last_row_is_the_budget_with_its_bar() {
        let budget = Some(dollars(300));

        let from_estimate = from_estimate(0, budget);
        let from_bill =
            MoneyViewModel::from_bill(&bill_of(12, 3), now(), budget, now());

        assert_eq!(
            from_estimate.rows,
            vec![
                row(Key::AgentsToday, "$2.50"),
                row(Key::AgentsLastSevenDays, "$2.50"),
                row(Key::AgentsThisMonth, "$3.75"),
                budget_row("$3.75", "$300.00", 1),
            ]
        );
        assert_eq!(
            from_bill.rows.last(),
            Some(&budget_row("$15.00", "$300.00", 5))
        );
    }

    #[test]
    fn requests_with_no_price_are_one_row_at_the_end() {
        let vm = from_estimate(3, None);

        assert_eq!(
            vm.note,
            Some(i18n::agents_template(
                Key::TplAgentsMoneyUnpriced,
                &[("count", "3")]
            ))
        );
        assert_eq!(vm.rows.len(), 3, "the rows stay as they are");
    }

    #[test]
    fn the_bill_lists_its_money_as_rows() {
        let two_minutes_ago = now() - Duration::from_secs(120);

        let vm = MoneyViewModel::from_bill(
            &bill_of(12, 3),
            two_minutes_ago,
            None,
            now(),
        );

        assert_eq!(
            vm.rows,
            vec![
                row(Key::AgentsToday, "$12.00"),
                row(Key::AgentsLastSevenDays, "$15.00"),
                row(Key::AgentsThisMonth, "$15.00"),
            ]
        );
        assert_eq!(vm.updated, Some(updated_text(now(), two_minutes_ago)));
        assert_eq!(vm.note, None);
    }

    #[test]
    fn a_failed_count_keeps_the_last_estimate_greyed_with_its_time() {
        let spend = spend_with(0);
        let money = estimate(&spend);
        let ten_minutes_ago = now() - Duration::from_secs(600);
        let mut spend = counted(spend, ten_minutes_ago);
        let error = SourceError::NotAvailable(String::from("no folder"));
        spend.apply(Some(Err(error)), now());

        let vm = MoneyViewModel::from_money(
            Some(&money),
            &spend,
            &good_prices(),
            None,
            now(),
        )
        .expect("the last estimate shows");

        assert!(vm.is_stale);
        assert_eq!(vm.rows.len(), 3);
        assert_eq!(
            vm.updated,
            Some(i18n::agents_template(
                Key::TplAgentsUpdated,
                &[("time", &i18n::with_count(Key::TplAgentsAgeMinutes, 10))]
            ))
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
    fn spending_over_the_budget_only_fills_the_bar() {
        let vm = MoneyViewModel::from_bill(
            &bill_of(400, 0),
            now(),
            Some(dollars(300)),
            now(),
        );

        assert_eq!(
            vm.rows.last(),
            Some(&budget_row("$400.00", "$300.00", 100))
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
                &good_prices(),
                None,
                now()
            ),
            None
        );
        assert_eq!(
            MoneyViewModel::from_money(
                None,
                &no_counter,
                &good_prices(),
                None,
                now()
            ),
            None
        );
    }

    /// The money rows of the tokens of [`spend_with`] with no estimate,
    /// as `prices` left them, against a budget.
    fn without_prices(prices: &ListPrices) -> MoneyViewModel {
        MoneyViewModel::from_money(
            None,
            &counted(spend_with(0), now()),
            prices,
            Some(dollars(300)),
            now(),
        )
        .expect("the rows show")
    }

    /// Owner, issue 37: "$0.00" would read as nothing spent.
    #[test]
    fn with_no_prices_each_amount_is_a_dash_with_a_line_under_them() {
        let vm = without_prices(&failed_prices());

        assert_eq!(
            vm.rows,
            vec![
                row(Key::AgentsToday, "—"),
                row(Key::AgentsLastSevenDays, "—"),
                row(Key::AgentsThisMonth, "—"),
            ],
            "the budget bar is left out"
        );
        assert_eq!(vm.note, Some(i18n::t(Key::AgentsNoPrices).to_string()));
        assert_eq!(vm.updated, Some(updated_text(now(), now())));
        assert!(!vm.is_stale);
    }

    #[test]
    fn while_the_prices_are_read_each_amount_is_a_dash_with_no_line() {
        let vm = without_prices(&ListPrices::default());

        assert_eq!(vm.rows.len(), 3);
        assert!(vm.rows.iter().all(|row| row.used == "—"));
        assert_eq!(vm.note, None);
    }
}
