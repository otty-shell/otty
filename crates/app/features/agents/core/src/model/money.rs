use std::time::SystemTime;

use otty_agents::{Bill, Dollars, MoneyEstimate};
use otty_app_i18n::{self as i18n, Key};

use super::super::prices::ListPrices;
use super::super::spend::ProfileSpend;
use super::UNKNOWN;
use super::kept_answer::KeptAnswer;
use super::limits::{UsageRowViewModel, updated_text};
use super::numbers::dollars_text;
use super::tokens::count_failed_text;

/// The Usage block of an API key profile's dashboard (spec Decisions 9, 10
/// and 12): when the money was read, then the money of today, 7 days and
/// this month as rows like the usage windows of a subscription, and the
/// requests with no price. There is no budget: OTTY only shows (spec
/// 2026-11-19, Decision 5). What
/// the money is, the bill or an estimate, is said only by the sources. The
/// last good estimate stays, greyed, when a later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// When the money was read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// Today, the last 7 days and this month; empty before a good count.
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
    /// organization read at `read_at`.
    pub(crate) fn from_bill(
        bill: &Bill,
        read_at: SystemTime,
        now: SystemTime,
    ) -> Self {
        Self {
            updated: Some(updated_text(now, read_at)),
            rows: money_rows([
                bill.today(),
                bill.last_7_days(),
                bill.this_month(),
            ]),
            note: None,
            is_stale: false,
            failure: None,
        }
    }

    /// The money of an API key profile at `now` from its `money` estimate
    /// and the count of `spend` it came from; `None` while there is
    /// nothing to show. While there are no `prices`, each amount is a
    /// dash, and a line says when no source gave them (spec Decision 16).
    pub(crate) fn from_money(
        money: Option<&MoneyEstimate>,
        spend: &ProfileSpend,
        prices: &ListPrices,
        now: SystemTime,
    ) -> Option<Self> {
        if !spend.has_counter() {
            return None;
        }

        let (counted_at, failure) =
            match KeptAnswer::new(spend.last_good(), spend.failure()) {
                KeptAnswer::Nothing => return None,
                KeptAnswer::OnlyFailure(error) => {
                    return Some(Self {
                        updated: None,
                        rows: Vec::new(),
                        note: None,
                        is_stale: false,
                        failure: Some(count_failed_text(error)),
                    });
                },
                KeptAnswer::Answer {
                    read_at, failure, ..
                } => (read_at, failure),
            };
        let updated = Some(updated_text(now, counted_at));
        let is_stale = failure.is_some();
        let failure = failure.map(count_failed_text);

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
            rows: money_rows(spent),
            note: (unpriced > 0).then(|| {
                i18n::with_count(Key::TplAgentsMoneyUnpriced, unpriced)
            }),
            is_stale,
            failure,
        })
    }
}

/// The rows of the money `spent` today, in the last 7 days and this month.
fn money_rows(spent: [Dollars; 3]) -> Vec<UsageRowViewModel> {
    let [today, last_7_days, this_month] = spent;
    let row = |name: Key, cost: Dollars| UsageRowViewModel {
        name: i18n::t(name).to_string(),
        used: dollars_text(cost),
        percent: None,
        resets: None,
    };

    vec![
        row(Key::AgentsToday, today),
        row(Key::AgentsLastSevenDays, last_7_days),
        row(Key::AgentsThisMonth, this_month),
    ]
}

/// The rows of today, the last 7 days and this month while there are no
/// prices: a dash in place of each amount, as "$0.00" would read as
/// nothing spent (spec Decision 16).
fn unknown_money_rows() -> Vec<UsageRowViewModel> {
    [
        Key::AgentsToday,
        Key::AgentsLastSevenDays,
        Key::AgentsThisMonth,
    ]
    .into_iter()
    .map(|name| UsageRowViewModel {
        name: i18n::t(name).to_string(),
        used: String::from(UNKNOWN),
        percent: None,
        resets: None,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use otty_agents::{
        DaySpend, ModelTokens, SourceError, SourceId, Spend, TokenCount,
        Unavailable,
    };

    use super::*;
    use crate::testing::{bill_of, now, price_table, prices_answer};

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
                DaySpend::new(jiff::civil::date(2026, 11, 2), vec![gpt5(1)]),
                DaySpend::new(jiff::civil::date(2026, 11, 16), today),
            ],
            Vec::new(),
        )
    }

    fn counted(spend: Spend, at: SystemTime) -> ProfileSpend {
        let mut counted = ProfileSpend::default();
        counted.apply(crate::types::SpendCount::Counted(spend), at);
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
                SourceError::NotAvailable(Unavailable::NoKeptPriceFile),
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

    /// The money of the estimate with `unpriced` requests.
    fn from_estimate(unpriced: u64) -> MoneyViewModel {
        let spend = spend_with(unpriced);
        let money = estimate(&spend);

        MoneyViewModel::from_money(
            Some(&money),
            &counted(spend, now()),
            &good_prices(),
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

    /// Spec 2026-11-19, Decision 5: there is no budget, so no bar.
    #[test]
    fn the_money_is_three_rows_with_no_bar() {
        let vm = from_estimate(0);

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
    fn requests_with_no_price_are_one_row_at_the_end() {
        let vm = from_estimate(3);

        assert_eq!(
            vm.note,
            Some(i18n::with_count(Key::TplAgentsMoneyUnpriced, 3))
        );
        assert_eq!(vm.rows.len(), 3, "the rows stay as they are");
    }

    #[test]
    fn the_bill_lists_its_money_as_rows() {
        let two_minutes_ago = now() - Duration::from_secs(120);

        let vm =
            MoneyViewModel::from_bill(&bill_of(12, 3), two_minutes_ago, now());

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
        let error = SourceError::NotAvailable(Unavailable::NoSessionFiles);
        spend.apply(crate::types::SpendCount::Failed(error), now());

        let vm = MoneyViewModel::from_money(
            Some(&money),
            &spend,
            &good_prices(),
            now(),
        )
        .expect("the last estimate shows");

        assert!(vm.is_stale);
        assert_eq!(vm.rows.len(), 3);
        assert_eq!(
            vm.updated,
            Some(i18n::filled(
                Key::TplAgentsUpdated,
                &[("time", &i18n::with_count(Key::TplMinutesAgo, 10))]
            ))
        );
        assert_eq!(
            vm.failure,
            Some(i18n::with_error(
                Key::TplAgentsTokensFailed,
                "not available: no session files"
            ))
        );
    }

    #[test]
    fn nothing_shows_before_a_count_or_without_a_counter() {
        let mut no_counter = ProfileSpend::default();
        no_counter.apply(crate::types::SpendCount::NoCounter, now());

        assert_eq!(
            MoneyViewModel::from_money(
                None,
                &ProfileSpend::default(),
                &good_prices(),
                now()
            ),
            None
        );
        assert_eq!(
            MoneyViewModel::from_money(
                None,
                &no_counter,
                &good_prices(),
                now()
            ),
            None
        );
    }

    /// The money rows of the tokens of [`spend_with`] with no estimate,
    /// as `prices` left them.
    fn without_prices(prices: &ListPrices) -> MoneyViewModel {
        MoneyViewModel::from_money(
            None,
            &counted(spend_with(0), now()),
            prices,
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
            ]
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
