//! Money as the pages show it. The Money block of an account page holds
//! what its provider told, which is exact. The money of the tokens of a
//! profile is a [`ShownMoney`], which knows what it is ([`MoneyKind`]):
//! at API prices, the tokens priced at the LiteLLM list prices, on a
//! subscription too, or exact, the cost the agent wrote (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3). The mark "at API
//! prices" and its tooltip come out of it, so no place adds a mark, a "~"
//! or a note itself.

use jiff::tz::TimeZone;
use otty_agents::{FetchedPrices, MoneyByKind, MoneyTotal, TokenCount};
use otty_app_i18n::{self as i18n, Key};

use super::breakdown::{Breakdown, BreakdownLine};
use super::limits::UsageRowViewModel;
use super::numbers::dollars_text;
use super::reset_time::date_time_text;

/// The Money block of an account page (spec 2026-11-19, Decision 5): when
/// the money was read, then its rows like the usage windows of a
/// subscription, and the money spent this month, as the provider told
/// them. There is no budget: OTTY only shows. The last good money stays,
/// greyed, when a later read fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoneyViewModel {
    /// When the money was read, such as "Updated 2 min ago".
    pub(crate) updated: Option<String>,
    /// What the key of the account told: Left, Key limit, Credits.
    pub(crate) rows: Vec<UsageRowViewModel>,
    /// The money spent this month, such as "$42.00": the bill or the
    /// provider's count; `None` when the provider told none.
    pub(crate) spent: Option<String>,
    /// What the agent's sessions on this computer spent this month at API
    /// prices, as the provider part counted it, for an account with no
    /// bill and no count of its provider (issue 22 of spec
    /// 2026-11-23-accounts-apart-from-profiles).
    pub(crate) spent_at_api_prices: Option<SpentAtApiPricesViewModel>,
    /// The row "Organization bill" of an organization (issue 24 of spec
    /// 2026-11-23-accounts-apart-from-profiles); `None` for an account that
    /// is not one, or whose bill could not be read, which `failure` tells.
    pub(crate) organization_bill: Option<OrganizationBillViewModel>,
    /// Whether the money is older, drawn greyed, because the latest read
    /// failed.
    pub(crate) is_stale: bool,
    /// Why the latest read failed, or why the bill could not be read.
    pub(crate) failure: Option<String>,
}

/// The bill of an organization this month, as its row shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrganizationBillViewModel {
    /// The bill read with the admin key, such as "$42.00".
    Read(String),
    /// No bill yet: "—", it shows once OTTY has an admin key.
    WaitsForAdminKey,
}

/// "Your spend this month" of an account: what the agent's own sessions
/// on this computer spent through it this month, priced at API prices by
/// its provider part, with the mark, its kinds on hover and where it was
/// counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpentAtApiPricesViewModel {
    /// The sum, an estimate: "~$42.10".
    pub(crate) amount: String,
    /// What the sum is made of, kind by kind, for its tooltip.
    pub(crate) kinds: Vec<BreakdownLine>,
    /// The mark "at API prices" with its tooltip.
    pub(crate) mark: ApiPricesMarkViewModel,
    /// Where it was counted, "From your Claude Code sessions on this
    /// computer"; `None` when the account names no profile of an agent.
    pub(crate) source: Option<String>,
}

impl SpentAtApiPricesViewModel {
    /// `amount` spent at API prices, made of `by_kind`, counted from the
    /// sessions of the agent named `agent`, priced at `prices`, fetched
    /// at a time told in the user's `time_zone`.
    pub(crate) fn of(
        amount: &str,
        by_kind: &MoneyByKind,
        agent: Option<&str>,
        prices: &FetchedPrices,
        time_zone: Option<&TimeZone>,
    ) -> Self {
        let money = MoneyTotal::at_api_prices(*by_kind, 1);
        let kinds =
            Breakdown::of(&TokenCount::default(), &money, Some(prices)).money;
        let source = agent.map(|agent| {
            i18n::filled(Key::TplAgentsSpentFromSessions, &[("agent", agent)])
        });

        Self {
            amount: format!("~{amount}"),
            kinds,
            mark: ApiPricesMarkViewModel::of(prices, time_zone),
            source,
        }
    }
}

/// A sum of money of tokens as it is shown: the sum written, such as
/// "$42.10", and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShownMoney {
    amount: String,
    kind: MoneyKind,
}

impl ShownMoney {
    /// The money of `total`, as the agent parts priced it at `prices`: at
    /// API prices when some of its requests were priced at them or had no
    /// price, exact when it is all the cost the agent wrote.
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
    /// "$42.10, at API prices".
    pub(crate) fn line_text(&self) -> String {
        match &self.kind {
            MoneyKind::AtApiPrices(mark) => mark.after(&self.amount),
            _ => self.amount.clone(),
        }
    }
}

#[cfg(test)]
impl ShownMoney {
    /// The sum written, such as "$42.10", with no mark.
    pub(crate) fn amount(&self) -> &str {
        &self.amount
    }

    /// What the sum is: at API prices with its mark, or exact.
    pub(crate) fn kind(&self) -> &MoneyKind {
        &self.kind
    }
}

/// What a shown sum of money of tokens is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MoneyKind {
    /// The cost the agent wrote, such as OpenCode does for each answer:
    /// exact, with no mark.
    Exact,
    /// Tokens priced at the LiteLLM list prices, with the mark "at API
    /// prices" and its tooltip.
    AtApiPrices(ApiPricesMarkViewModel),
}

impl MoneyKind {
    /// The mark of money at API prices; `None` for exact money.
    pub(crate) fn mark(&self) -> Option<&ApiPricesMarkViewModel> {
        match self {
            Self::AtApiPrices(mark) => Some(mark),
            _ => None,
        }
    }

    /// The kind of money counted at `prices`: at API prices when
    /// `is_at_api_prices` (some tokens were priced at the list prices),
    /// else exact. The tooltip tells when the prices were fetched, in the
    /// user's `time_zone`.
    pub(crate) fn at_prices(
        is_at_api_prices: bool,
        prices: &FetchedPrices,
        time_zone: Option<&TimeZone>,
    ) -> Self {
        if !is_at_api_prices {
            return Self::Exact;
        }

        Self::AtApiPrices(ApiPricesMarkViewModel::of(prices, time_zone))
    }
}

/// The short mark next to money at API prices (spec
/// 2026-11-23-accounts-apart-from-profiles, Decision 3): "at API prices",
/// with a tooltip that says the tokens are priced at the API list prices
/// fetched at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiPricesMarkViewModel {
    /// The mark: "at API prices".
    pub(crate) label: &'static str,
    /// Its tooltip: the tokens priced at the API list prices fetched at a
    /// time.
    pub(crate) tip: String,
}

impl ApiPricesMarkViewModel {
    /// The mark of money at `prices`, its tooltip with when the prices
    /// were fetched, in the user's `time_zone`.
    fn of(prices: &FetchedPrices, time_zone: Option<&TimeZone>) -> Self {
        let time = date_time_text(prices.fetched_at(), time_zone);

        Self {
            label: i18n::t(Key::AgentsApiPricesMark),
            tip: i18n::filled(
                Key::TplAgentsMoneyAtApiPrices,
                &[("time", &time)],
            ),
        }
    }

    /// `amount` followed by the mark: "$42.10, at API prices".
    fn after(&self, amount: &str) -> String {
        i18n::filled(
            Key::TplAgentsAmountAtApiPrices,
            &[("amount", amount), ("mark", self.label)],
        )
    }
}

#[cfg(test)]
mod tests {
    use otty_agents::{Dollars, ModelTokens, TokenCount};

    use super::*;
    use crate::testing::{at, fetched_prices, input_money};

    /// The tooltip of the mark for prices fetched at `at(10)`.
    fn tip() -> String {
        let time = date_time_text(at(10), None);
        i18n::filled(Key::TplAgentsMoneyAtApiPrices, &[("time", &time)])
    }

    fn with_mark(amount: &str) -> String {
        i18n::filled(
            Key::TplAgentsAmountAtApiPrices,
            &[
                ("amount", amount),
                ("mark", i18n::t(Key::AgentsApiPricesMark)),
            ],
        )
    }

    /// One million input tokens of gpt-5 that cost `money` as their agent
    /// part priced them.
    fn gpt_5(money: MoneyTotal) -> ModelTokens {
        ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000_000, 0, 0), 1)
            .with_money(money)
    }

    /// Spec 2026-11-23, Decision 3: tokens the agent part priced at API
    /// prices carry the mark "at API prices", under the sum or after it in one line,
    /// and its tooltip tells when the prices were fetched.
    #[test]
    fn money_at_api_prices_shows_its_mark() {
        let prices = fetched_prices(at(10));
        let listed = MoneyTotal::at_api_prices(input_money(1_250_000), 1);
        let total = MoneyTotal::of_models(&[gpt_5(listed)]);

        let money = ShownMoney::of_total(&total, &prices, None);

        assert_eq!(money.amount(), "$1.25", "no \"~\" in the sum");
        let mark = money.kind().mark().expect("the money has its mark");
        assert_eq!(mark.label, i18n::t(Key::AgentsApiPricesMark));
        assert_eq!(mark.tip, tip());
        assert_eq!(money.line_text(), with_mark("$1.25"));
    }

    /// Issue 01 of spec 2026-11-23 keeps it: the cost the agent wrote is
    /// exact, with no mark.
    #[test]
    fn the_cost_the_agent_wrote_is_exact() {
        let prices = fetched_prices(at(10));
        let written = MoneyTotal::written(Dollars::from_micro_dollars(500_000));
        let total = MoneyTotal::of_models(&[gpt_5(written)]);

        let money = ShownMoney::of_total(&total, &prices, None);

        assert_eq!(money.amount(), "$0.50");
        assert_eq!(money.kind(), &MoneyKind::Exact);
        assert_eq!(money.line_text(), "$0.50");
    }

    #[test]
    fn money_is_at_api_prices_only_when_told_so() {
        let prices = fetched_prices(at(10));

        assert_eq!(
            MoneyKind::at_prices(false, &prices, None),
            MoneyKind::Exact
        );
        assert!(MoneyKind::at_prices(true, &prices, None).mark().is_some());
    }
}
