//! Money as the pages show it. Owner, 2026-11-21: one type for every sum
//! that is shown, [`ShownMoney`], which knows what it is ([`MoneyKind`]):
//! exact, or an estimate at the LiteLLM list prices. The mark of an
//! estimate, "estimate, list prices", and its tooltip come out of it, so
//! no place adds a mark, a "~" or a note itself. Where the room is tight,
//! an estimate shows only its sum, with the tooltip of the mark.

use jiff::tz::TimeZone;
use otty_agents::{FetchedPrices, MoneyTotal};
use otty_app_i18n::{self as i18n, Key};

use super::limits::UsageRowViewModel;
use super::numbers::dollars_text;
use super::reset_time::date_time_text;

/// The Money block of an account page (spec 2026-11-19, Decision 5): when
/// the money was read, then its rows like the usage windows of a
/// subscription, the money spent this month, and a muted line. There is
/// no budget: OTTY only shows. The last good money stays, greyed, when a
/// later count fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// When the money was read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// What the key of the account told: Left, Key limit, Credits.
    pub(crate) rows: Vec<UsageRowViewModel>,
    /// The money spent this month: the bill, the provider's count or an
    /// estimate, which brings its own mark; `None` before a good count.
    pub(crate) spent: Option<ShownMoney>,
    /// The muted line under the rows: how many requests of this month
    /// have no price, or that no prices could be read.
    pub(crate) note: Option<String>,
    /// Whether the money is an older estimate, drawn greyed, because the
    /// latest count failed.
    pub(crate) is_stale: bool,
    /// Why the latest count failed, or why the bill could not be read.
    pub(crate) failure: Option<String>,
}

/// A sum of money as it is shown: the sum written, such as "$42.10", and
/// what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShownMoney {
    amount: String,
    kind: MoneyKind,
}

impl ShownMoney {
    /// The sum written, such as "$42.10", with no mark.
    pub(crate) fn amount(&self) -> &str {
        &self.amount
    }

    /// What the sum is: exact, or an estimate with its mark.
    pub(crate) fn kind(&self) -> &MoneyKind {
        &self.kind
    }

    /// Exact money, such as a bill or the provider's own count: `amount`
    /// is already written, such as "$42.00" or "¥110.00", or a dash.
    pub(crate) fn exact(amount: String) -> Self {
        Self {
            amount,
            kind: MoneyKind::Exact,
        }
    }

    /// The money of `total`, counted at `prices`: an estimate when some of
    /// its requests were priced at the list prices or had no price, exact
    /// when it is all the cost the agent wrote.
    pub(crate) fn of_total(
        total: &MoneyTotal,
        prices: &FetchedPrices,
        time_zone: Option<&TimeZone>,
    ) -> Self {
        Self {
            amount: dollars_text(total.cost()),
            kind: MoneyKind::at_prices(total.is_estimate(), prices, time_zone),
        }
    }

    /// The sum in one line, where there is room for the mark but no place
    /// for it under the sum, such as the Cost of a project: "$42.10", or
    /// "$42.10 · estimate, list prices" for an estimate.
    pub(crate) fn line_text(&self) -> String {
        match &self.kind {
            MoneyKind::Estimate(mark) => mark.after(&self.amount),
            _ => self.amount.clone(),
        }
    }

    /// The tight form of money spent this month, such as a row of the left
    /// panel, a chip or a table cell: "$44.87 this month", or only the sum,
    /// "$42.10", for an estimate (owner, 2026-11-21, issue 27). The tooltip
    /// of the mark, [`Self::tip`], still says it is an estimate.
    pub(crate) fn month_short_text(&self) -> String {
        match &self.kind {
            MoneyKind::Estimate(_) => self.amount.clone(),
            _ => i18n::filled(
                Key::TplAgentsAmountThisMonth,
                &[("amount", &self.amount)],
            ),
        }
    }

    /// The tooltip of the tight form: the tooltip of the mark of an
    /// estimate; `None` for exact money.
    pub(crate) fn tip(&self) -> Option<&str> {
        self.kind.mark().map(|mark| mark.tip.as_str())
    }
}

/// What a shown sum of money is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MoneyKind {
    /// The provider's bill or own count, or the cost the agent wrote:
    /// exact, with no mark.
    Exact,
    /// Tokens priced at the LiteLLM list prices: an estimate, with the
    /// mark "estimate, list prices" and its tooltip.
    Estimate(EstimateMarkViewModel),
}

impl MoneyKind {
    /// The mark of an estimate; `None` for exact money.
    pub(crate) fn mark(&self) -> Option<&EstimateMarkViewModel> {
        match self {
            Self::Estimate(mark) => Some(mark),
            _ => None,
        }
    }

    /// The kind of money counted at `prices`: an estimate when
    /// `is_estimate` (some tokens were priced at the list prices), else
    /// exact. The tooltip tells when the prices were fetched, in the
    /// user's `time_zone`.
    pub(crate) fn at_prices(
        is_estimate: bool,
        prices: &FetchedPrices,
        time_zone: Option<&TimeZone>,
    ) -> Self {
        if !is_estimate {
            return Self::Exact;
        }

        Self::Estimate(EstimateMarkViewModel::of(prices, time_zone))
    }
}

/// The short mark next to money that is an estimate at the LiteLLM list
/// prices (issue 19 of spec 2026-11-19-agent-plugins): "estimate, list
/// prices", with a tooltip that says when the prices are from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EstimateMarkViewModel {
    /// The mark: "estimate, list prices".
    pub(crate) label: &'static str,
    /// Its tooltip: the tokens of the session files at the LiteLLM prices
    /// of a time, an estimate.
    pub(crate) tip: String,
}

impl EstimateMarkViewModel {
    /// The mark of money estimated at `prices`, its tooltip with when the
    /// prices were fetched, in the user's `time_zone`.
    fn of(prices: &FetchedPrices, time_zone: Option<&TimeZone>) -> Self {
        let time = date_time_text(prices.fetched_at(), time_zone);

        Self {
            label: i18n::t(Key::AgentsEstimateMark),
            tip: i18n::filled(Key::TplAgentsMoneyEstimate, &[("time", &time)]),
        }
    }

    /// `amount` followed by the mark: "$42.10 · estimate, list prices".
    fn after(&self, amount: &str) -> String {
        i18n::filled(
            Key::TplAgentsEstimatedAmount,
            &[("amount", amount), ("mark", self.label)],
        )
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{Dollars, ModelTokens, Registry, TokenCount};

    use super::*;
    use crate::testing::{at, fetched_prices, price_table};

    /// The tooltip of the mark for prices fetched at `at(10)`.
    fn tip() -> String {
        let time = date_time_text(at(10), None);
        i18n::filled(Key::TplAgentsMoneyEstimate, &[("time", &time)])
    }

    fn with_mark(amount: &str) -> String {
        i18n::filled(
            Key::TplAgentsEstimatedAmount,
            &[
                ("amount", amount),
                ("mark", i18n::t(Key::AgentsEstimateMark)),
            ],
        )
    }

    /// One million input tokens of gpt-5 ($1.25 at the made-up prices),
    /// with the cost the agent wrote when `written` is given.
    fn gpt_5(written: Option<u64>) -> ModelTokens {
        ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000_000, 0, 0), 1)
            .with_cost(written.map(Dollars::from_micro_dollars))
    }

    #[test]
    fn a_bill_shows_no_mark_in_either_form() {
        let bill = ShownMoney::exact(String::from("$42.00"));

        assert_eq!(bill.amount(), "$42.00");
        assert_eq!(bill.kind().mark(), None);
        assert_eq!(bill.line_text(), "$42.00");
        assert_eq!(
            bill.month_short_text(),
            i18n::filled(
                Key::TplAgentsAmountThisMonth,
                &[("amount", "$42.00")]
            )
        );
        assert_eq!(bill.tip(), None);
    }

    /// Owner, 2026-11-21: one mark everywhere. The full form keeps the sum
    /// with no "~" and puts the mark under it, or after it in one line; the
    /// tight form is only the sum (issue 27), with the tooltip of the mark.
    /// The tooltip tells when the prices were fetched.
    #[test]
    fn an_estimate_shows_the_mark_where_there_is_room_and_only_the_sum_where_tight()
     {
        let prices = fetched_prices(at(10));
        let total = MoneyTotal::of_models(
            &[gpt_5(None)],
            prices.table(),
            &Registry::default(),
        );

        let money = ShownMoney::of_total(&total, &prices, None);

        assert_eq!(money.amount(), "$1.25", "no \"~\" in the sum");
        let mark = money.kind().mark().expect("an estimate has its mark");
        assert_eq!(mark.label, i18n::t(Key::AgentsEstimateMark));
        assert_eq!(mark.tip, tip());
        assert_eq!(money.line_text(), with_mark("$1.25"));
        assert_eq!(money.month_short_text(), "$1.25");
        assert_eq!(money.tip(), Some(tip().as_str()));
    }

    #[test]
    fn the_cost_the_agent_wrote_is_exact() {
        let prices = fetched_prices(at(10));
        let total = MoneyTotal::of_models(
            &[gpt_5(Some(500_000))],
            &price_table(),
            &Registry::default(),
        );

        let money = ShownMoney::of_total(&total, &prices, None);

        assert_eq!(money.amount(), "$0.50");
        assert_eq!(money.kind(), &MoneyKind::Exact);
        assert_eq!(money.tip(), None);
    }

    #[test]
    fn money_is_an_estimate_only_when_told_so() {
        let prices = fetched_prices(at(10));

        assert_eq!(
            MoneyKind::at_prices(false, &prices, None),
            MoneyKind::Exact
        );
        assert!(MoneyKind::at_prices(true, &prices, None).mark().is_some());
    }
}
