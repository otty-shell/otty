//! The money of the tokens of a profile (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3). The agent part
//! prices each answer itself: the cost the agent wrote, as it is, else the
//! tokens at the API price of the model that OTTY's price list gave,
//! marked "at API prices", on a subscription too, and kind by kind (issue
//! 16). OTTY only adds the money of the answers up. Money at API prices is
//! an estimate, not a bill: it is as old as the list's prices, and knows
//! nothing of discounts or of the fast mode.

use std::ops::Add;

use crate::dollars::Dollars;
use crate::spend::ModelTokens;
use crate::token_count::TokenCount;

/// What one answer cost, as its agent part priced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnswerCost {
    /// The cost the agent wrote itself, exact, with no parts.
    Written(Dollars),
    /// The tokens at the API price of the model, an estimate, kind by
    /// kind.
    AtApiPrices(MoneyByKind),
}

/// Money at API prices by the kind of token it paid for: the input not
/// written to the cache, the input written to the cache, the tokens read
/// from the cache and the output. The pages list it on hover, so a big
/// count of cheap cached tokens tells what it costs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoneyByKind {
    input: Dollars,
    cache_written: Dollars,
    cached: Dollars,
    output: Dollars,
}

impl MoneyByKind {
    /// The money of the input not written to the cache.
    pub fn input(&self) -> Dollars {
        self.input
    }

    /// The money of the input written to the cache.
    pub fn cache_written(&self) -> Dollars {
        self.cache_written
    }

    /// The money of the tokens read from the cache.
    pub fn cached(&self) -> Dollars {
        self.cached
    }

    /// The money of the output.
    pub fn output(&self) -> Dollars {
        self.output
    }

    /// The four kinds added up.
    ///
    /// ```
    /// use otty_agents::{Dollars, MoneyByKind};
    ///
    /// let micro = Dollars::from_micro_dollars;
    /// let money = MoneyByKind::new(micro(10), micro(20), micro(3), micro(40));
    ///
    /// assert_eq!(money.total(), micro(73));
    /// ```
    pub fn total(&self) -> Dollars {
        self.input + self.cache_written + self.cached + self.output
    }

    /// Money of each kind: `input`, `cache_written`, `cached` and
    /// `output`.
    pub fn new(
        input: Dollars,
        cache_written: Dollars,
        cached: Dollars,
        output: Dollars,
    ) -> Self {
        Self {
            input,
            cache_written,
            cached,
            output,
        }
    }
}

impl Add for MoneyByKind {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            input: self.input + other.input,
            cache_written: self.cache_written + other.cache_written,
            cached: self.cached + other.cached,
            output: self.output + other.output,
        }
    }
}

/// The money of a stretch of time: the cost the agent wrote, the money at
/// API prices by kind, how many requests had no price, and how many were
/// priced at API prices instead of the cost the agent wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoneyTotal {
    written: Dollars,
    priced: MoneyByKind,
    unpriced_requests: u64,
    requests_at_api_prices: u64,
}

impl MoneyTotal {
    /// What the requests with a price cost: the cost the agent wrote and
    /// the money at API prices.
    pub fn cost(&self) -> Dollars {
        self.written + self.priced.total()
    }

    /// The share of [`cost`](Self::cost) the agent wrote, which has no
    /// parts.
    pub fn written_cost(&self) -> Dollars {
        self.written
    }

    /// The share of [`cost`](Self::cost) at API prices, kind by kind.
    pub fn priced_by_kind(&self) -> MoneyByKind {
        self.priced
    }

    /// How many requests had no price: their money is not in
    /// [`cost`](Self::cost).
    pub fn unpriced_requests(&self) -> u64 {
        self.unpriced_requests
    }

    /// Whether the money is an estimate, shown with the mark "at API
    /// prices": a request was priced at API prices or had no price. Money
    /// made only of the cost the agent wrote, or of nothing spent, is
    /// exact.
    ///
    /// ```
    /// use otty_agents::{Dollars, MoneyByKind, MoneyTotal};
    ///
    /// let micro = Dollars::from_micro_dollars;
    /// let written = MoneyTotal::written(micro(12));
    /// let listed = MoneyTotal::at_api_prices(
    ///     MoneyByKind::new(micro(5), micro(0), micro(0), micro(0)),
    ///     1,
    /// );
    ///
    /// assert!(!written.is_estimate());
    /// assert!((written + listed).is_estimate());
    /// assert_eq!((written + listed).cost().micro_dollars(), 17);
    /// ```
    pub fn is_estimate(&self) -> bool {
        self.requests_at_api_prices > 0 || self.unpriced_requests > 0
    }

    /// Requests whose cost the agent wrote, `cost` in all: exact money.
    pub fn written(cost: Dollars) -> Self {
        Self {
            written: cost,
            ..Self::default()
        }
    }

    /// `requests` requests the agent part priced at API prices, `money`
    /// in all, kind by kind.
    pub fn at_api_prices(money: MoneyByKind, requests: u64) -> Self {
        Self {
            priced: money,
            requests_at_api_prices: requests,
            ..Self::default()
        }
    }

    /// `requests` requests with no price: neither the agent wrote what
    /// they cost nor the price list has their model.
    pub fn unpriced(requests: u64) -> Self {
        Self {
            unpriced_requests: requests,
            ..Self::default()
        }
    }

    /// The money of the tokens of `models`, each with the money its agent
    /// part gave ([`ModelTokens::money`]).
    ///
    /// ```
    /// use otty_agents::{
    ///     Dollars, ModelTokens, MoneyByKind, MoneyTotal, TokenCount,
    /// };
    ///
    /// let micro = Dollars::from_micro_dollars;
    /// let priced = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
    ///     .with_money(MoneyTotal::at_api_prices(
    ///         MoneyByKind::new(micro(40), micro(0), micro(0), micro(0)),
    ///         1,
    ///     ));
    /// let unpriced =
    ///     ModelTokens::new(Some("mystery"), TokenCount::new(5, 0, 0), 2)
    ///         .with_money(MoneyTotal::unpriced(2));
    ///
    /// let total = MoneyTotal::of_models(&[priced, unpriced]);
    ///
    /// assert_eq!(total.cost().micro_dollars(), 40);
    /// assert_eq!(total.unpriced_requests(), 2);
    /// ```
    pub fn of_models(models: &[ModelTokens]) -> Self {
        models
            .iter()
            .map(ModelTokens::money)
            .fold(Self::default(), |sum, model| sum + model)
    }

    /// The money of one answer of `tokens` that cost `cost` as its part
    /// priced it. An answer with no cost and no tokens costs nothing:
    /// there is nothing to price.
    pub(crate) fn of_answer(
        cost: Option<AnswerCost>,
        tokens: TokenCount,
    ) -> Self {
        match cost {
            Some(AnswerCost::Written(cost)) => Self::written(cost),
            Some(AnswerCost::AtApiPrices(money)) => {
                Self::at_api_prices(money, 1)
            },
            None if tokens.is_zero() => Self::default(),
            None => Self::unpriced(1),
        }
    }
}

impl Add for MoneyTotal {
    type Output = Self;

    /// The sum of two totals: the written costs added up, the money at
    /// API prices kind by kind, and the requests.
    fn add(self, other: Self) -> Self {
        Self {
            written: self.written + other.written,
            priced: self.priced + other.priced,
            unpriced_requests: self
                .unpriced_requests
                .saturating_add(other.unpriced_requests),
            requests_at_api_prices: self
                .requests_at_api_prices
                .saturating_add(other.requests_at_api_prices),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dollars(micro_dollars: u64) -> Dollars {
        Dollars::from_micro_dollars(micro_dollars)
    }

    fn tokens(input: u64) -> TokenCount {
        TokenCount::new(input, 0, 0)
    }

    /// Money at API prices of `input` millionths of a dollar, all of it
    /// for input tokens.
    fn input_only(input: u64) -> MoneyByKind {
        MoneyByKind::new(dollars(input), dollars(0), dollars(0), dollars(0))
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: the money
    /// of each answer is what its part gave, written or at API prices, and
    /// OTTY adds it up.
    #[test]
    fn the_money_of_each_answer_is_added_up() {
        let answers = [
            MoneyTotal::of_answer(
                Some(AnswerCost::Written(dollars(250))),
                tokens(10),
            ),
            MoneyTotal::of_answer(
                Some(AnswerCost::AtApiPrices(input_only(1_000))),
                tokens(10),
            ),
            MoneyTotal::of_answer(None, tokens(10)),
        ];

        let total = answers
            .into_iter()
            .fold(MoneyTotal::default(), |sum, answer| sum + answer);

        assert_eq!(total.cost(), dollars(1_250));
        assert_eq!(total.unpriced_requests(), 1);
        assert!(total.is_estimate());
    }

    /// Issue 16 of spec 2026-11-23: money at API prices keeps its money
    /// by kind, the sum adds the kinds up one by one, and the kinds add
    /// up to the money at API prices.
    #[test]
    fn money_at_api_prices_adds_up_kind_by_kind() {
        let first = MoneyByKind::new(
            dollars(10),
            dollars(20),
            dollars(30),
            dollars(40),
        );
        let second =
            MoneyByKind::new(dollars(1), dollars(0), dollars(3), dollars(0));

        let total = MoneyTotal::of_answer(
            Some(AnswerCost::AtApiPrices(first)),
            tokens(10),
        ) + MoneyTotal::of_answer(
            Some(AnswerCost::AtApiPrices(second)),
            tokens(10),
        );

        let by_kind = total.priced_by_kind();
        assert_eq!(
            by_kind,
            MoneyByKind::new(
                dollars(11),
                dollars(20),
                dollars(33),
                dollars(40)
            )
        );
        assert_eq!(by_kind.total(), dollars(104));
        assert_eq!(total.cost(), dollars(104));
        assert_eq!(total.written_cost(), Dollars::default());
    }

    /// Issue 16: the cost the agent wrote has no parts; next to money at
    /// API prices it stays apart, and the cost adds both.
    #[test]
    fn the_cost_the_agent_wrote_has_no_parts() {
        let written = MoneyTotal::of_answer(
            Some(AnswerCost::Written(dollars(250))),
            tokens(10),
        );
        let mix = written
            + MoneyTotal::of_answer(
                Some(AnswerCost::AtApiPrices(input_only(100))),
                tokens(10),
            );

        assert_eq!(written.priced_by_kind(), MoneyByKind::default());
        assert_eq!(written.written_cost(), dollars(250));
        assert_eq!(mix.written_cost(), dollars(250));
        assert_eq!(mix.priced_by_kind(), input_only(100));
        assert_eq!(mix.cost(), dollars(350));
    }

    #[test]
    fn money_made_only_of_written_costs_is_exact() {
        let written = MoneyTotal::of_answer(
            Some(AnswerCost::Written(dollars(250))),
            tokens(10),
        );

        assert!(!written.is_estimate());
        assert!(
            !MoneyTotal::default().is_estimate(),
            "nothing spent is exact"
        );
    }

    #[test]
    fn money_with_a_request_at_api_prices_or_with_no_price_is_an_estimate() {
        let listed = MoneyTotal::of_answer(
            Some(AnswerCost::AtApiPrices(input_only(1))),
            tokens(10),
        );
        let unpriced = MoneyTotal::of_answer(None, tokens(10));

        assert!(listed.is_estimate());
        assert!(unpriced.is_estimate());
    }

    #[test]
    fn an_answer_with_no_tokens_costs_nothing_even_with_no_price() {
        let total = MoneyTotal::of_answer(None, TokenCount::default());

        assert_eq!(total, MoneyTotal::default());
        assert!(!total.is_estimate());
    }

    #[test]
    fn the_money_of_models_is_the_sum_of_their_money() {
        let models = [
            ModelTokens::new(Some("a"), tokens(10), 2)
                .with_money(MoneyTotal::at_api_prices(input_only(300), 2)),
            ModelTokens::new(Some("b"), tokens(10), 1)
                .with_money(MoneyTotal::written(dollars(200))),
            ModelTokens::new(Some("c"), tokens(10), 3)
                .with_money(MoneyTotal::unpriced(3)),
        ];

        let total = MoneyTotal::of_models(&models);

        assert_eq!(total.cost(), dollars(500));
        assert_eq!(total.priced_by_kind(), input_only(300));
        assert_eq!(total.written_cost(), dollars(200));
        assert_eq!(total.unpriced_requests(), 3);
        assert!(total.is_estimate());
    }
}
