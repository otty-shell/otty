//! The money of the tokens of a profile at API prices (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3): each model at the
//! LiteLLM list prices, on a subscription too, unless the agent wrote what
//! an answer cost. It is an estimate, not a bill: it is as old as the
//! table's prices, and knows nothing of discounts or of the fast mode.

use std::ops::Add;

use crate::dollars::Dollars;
use crate::price_table::PriceTable;
use crate::registry::Registry;
use crate::spend::ModelTokens;

/// The money of a stretch of time: what the priced requests cost, how
/// many requests had no price, and how many were priced at the list prices
/// instead of the cost the agent wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoneyTotal {
    cost: Dollars,
    unpriced_requests: u64,
    listed_requests: u64,
}

impl MoneyTotal {
    /// What the requests with a price cost.
    pub fn cost(&self) -> Dollars {
        self.cost
    }

    /// How many requests had no price: their money is not in
    /// [`cost`](Self::cost).
    pub fn unpriced_requests(&self) -> u64 {
        self.unpriced_requests
    }

    /// Whether the money is an estimate, shown with the mark "at API
    /// prices": a request was priced at the list prices or had no price.
    /// Money made only of the cost the agent wrote, or of nothing spent, is
    /// exact.
    ///
    /// ```
    /// use otty_agents::{
    ///     Dollars, ModelTokens, MoneyTotal, PriceTable, Registry, TokenCount,
    /// };
    ///
    /// let written = ModelTokens::new(Some("gpt-5"), TokenCount::new(10, 0, 0), 1)
    ///     .with_cost(Some(Dollars::from_micro_dollars(12)));
    ///
    /// let total = MoneyTotal::of_models(
    ///     &[written],
    ///     &PriceTable::new(Vec::new()),
    ///     &Registry::default(),
    /// );
    ///
    /// assert!(!total.is_estimate());
    /// assert_eq!(total.cost().micro_dollars(), 12);
    /// ```
    pub fn is_estimate(&self) -> bool {
        self.listed_requests > 0 || self.unpriced_requests > 0
    }

    /// What the tokens of `models` cost: the cost the agent wrote when it
    /// did, else the list `prices`. A model with no price keeps its tokens
    /// and adds its requests as having no price, so the screen can say the
    /// money is at least the sum shown. The requests with a long prompt are
    /// priced at the long prompt prices of their model.
    ///
    /// The tokens of a model are priced at the row of who served them,
    /// else at the maker's row (see [`PriceTable::price_of`]). Who served
    /// them is named as `registry` names the provider to the price table
    /// (see [`Registry::price_name_of`]).
    ///
    /// ```
    /// use otty_agents::{
    ///     Dollars, ModelPrice, ModelTokens, MoneyTotal, PriceTable, Registry,
    ///     TokenCount, TokenPrices,
    /// };
    ///
    /// let gpt_5 = TokenPrices {
    ///     input: Dollars::from_micro_dollars(1_250_000),
    ///     ..TokenPrices::default()
    /// };
    /// let prices = PriceTable::new(vec![ModelPrice::new("gpt-5", gpt_5)]);
    /// let models = [
    ///     ModelTokens::new(Some("gpt-5"), TokenCount::new(1_000_000, 0, 0), 1),
    ///     ModelTokens::new(Some("mystery"), TokenCount::new(5, 0, 0), 2),
    /// ];
    ///
    /// let total = MoneyTotal::of_models(&models, &prices, &Registry::default());
    ///
    /// assert_eq!(total.cost().micro_dollars(), 1_250_000);
    /// assert_eq!(total.unpriced_requests(), 2);
    /// ```
    pub fn of_models(
        models: &[ModelTokens],
        prices: &PriceTable,
        registry: &Registry,
    ) -> Self {
        models
            .iter()
            .map(|model| model_total(model, prices, registry))
            .fold(Self::default(), |sum, model| sum + model)
    }
}

impl Add for MoneyTotal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            cost: self.cost + other.cost,
            unpriced_requests: self
                .unpriced_requests
                .saturating_add(other.unpriced_requests),
            listed_requests: self
                .listed_requests
                .saturating_add(other.listed_requests),
        }
    }
}

/// What the tokens of `model` cost: the cost the agent wrote, else their
/// cost at `prices` for who served them, else their requests as having no
/// price. A model with no tokens costs nothing, even with no price: there
/// is nothing to price.
fn model_total(
    model: &ModelTokens,
    prices: &PriceTable,
    registry: &Registry,
) -> MoneyTotal {
    if let Some(cost) = model.cost() {
        return MoneyTotal {
            cost,
            ..MoneyTotal::default()
        };
    }
    if model.tokens().is_zero() {
        return MoneyTotal::default();
    }

    let served_by = model
        .provider()
        .and_then(|provider| registry.price_name_of(provider));
    let price = model
        .model()
        .and_then(|name| prices.price_of(name, served_by));

    match price {
        Some(price) => MoneyTotal {
            cost: price.cost(model),
            unpriced_requests: 0,
            listed_requests: model.requests(),
        },
        None => MoneyTotal {
            cost: Dollars::default(),
            unpriced_requests: model.requests(),
            listed_requests: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::litellm::read_litellm;
    use crate::price_table::ModelPrice;
    use crate::provider::{ProviderEntry, ProviderId, ProviderRef};
    use crate::registry::Registry;
    use crate::spend_files::{CountedAnswer, FileSpend};
    use crate::spend_tally::tally;
    use crate::testing::{
        NOW, at, dollars, made_up_part, token_prices, utc_now,
    };
    use crate::token_count::TokenCount;

    /// A dollar in micro-dollars.
    const DOLLAR: u64 = 1_000_000;

    /// Made-up prices: `m` costs $1 per million input tokens and $10 per
    /// million output tokens.
    fn prices() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new(
                "m",
                token_prices(DOLLAR, DOLLAR / 10, 2 * DOLLAR, 10 * DOLLAR),
            ),
            ModelPrice::new(
                "claude-opus-4-5",
                token_prices(5 * DOLLAR, 0, 0, 0),
            ),
        ])
    }

    fn million_in(model: Option<&str>, requests: u64) -> ModelTokens {
        ModelTokens::new(model, TokenCount::new(1_000_000, 0, 0), requests)
    }

    #[test]
    fn models_are_priced_and_a_model_with_no_price_is_counted_apart() {
        let models = [
            million_in(Some("m"), 2),
            million_in(Some("claude-opus-4-5"), 1),
            million_in(None, 3),
            million_in(Some("mystery"), 4),
            ModelTokens::new(Some("mystery"), TokenCount::default(), 9),
        ];

        let total =
            MoneyTotal::of_models(&models, &prices(), &Registry::default());

        assert_eq!(total.cost(), dollars(6));
        assert_eq!(total.unpriced_requests(), 7);
    }

    #[test]
    fn the_cost_the_agent_wrote_is_used_instead_of_the_list_prices() {
        let models = [
            million_in(Some("m"), 2)
                .with_cost(Some(Dollars::from_micro_dollars(250_000))),
            million_in(Some("mystery"), 1).with_cost(Some(dollars(1))),
        ];

        let total =
            MoneyTotal::of_models(&models, &prices(), &Registry::default());

        assert_eq!(total.cost(), Dollars::from_micro_dollars(1_250_000));
        assert_eq!(total.unpriced_requests(), 0);
        assert!(!total.is_estimate());
    }

    #[test]
    fn money_with_a_request_at_the_list_prices_or_with_no_price_is_an_estimate()
    {
        let listed = [
            million_in(Some("m"), 1),
            million_in(Some("x"), 1).with_cost(Some(dollars(1))),
        ];
        let unpriced = [million_in(Some("mystery"), 1)];

        assert!(
            MoneyTotal::of_models(&listed, &prices(), &Registry::default())
                .is_estimate()
        );
        assert!(
            MoneyTotal::of_models(&unpriced, &prices(), &Registry::default())
                .is_estimate()
        );
        assert!(
            !MoneyTotal::default().is_estimate(),
            "nothing spent is exact"
        );
    }

    #[test]
    fn a_dated_snapshot_and_cache_reads_are_priced() {
        let models = [
            ModelTokens::new(
                Some("m"),
                TokenCount::new(1_000_000, 1_000_000, 0),
                1,
            ),
            million_in(Some("claude-opus-4-5-20251101"), 1),
        ];

        let total =
            MoneyTotal::of_models(&models, &prices(), &Registry::default());

        // `m`: $1 of input and $0.10 of cache reads; Opus: $5 of input.
        assert_eq!(total.cost(), Dollars::from_micro_dollars(6_100_000));
    }

    #[test]
    fn a_model_with_no_tokens_costs_nothing_even_with_no_price() {
        let models = [ModelTokens::new(
            Some("<synthetic>"),
            TokenCount::default(),
            4,
        )];

        let total =
            MoneyTotal::of_models(&models, &prices(), &Registry::default());

        assert_eq!(total, MoneyTotal::default());
        assert!(!total.is_estimate());
    }

    /// Made-up prices of models that cost more for a long prompt, as the
    /// LiteLLM table has them: Haiku over 100,000 tokens, Sol over
    /// 272,000.
    fn long_prompt_prices() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new(
                "claude-haiku-5-5",
                token_prices(100_000, 10_000, 125_000, 500_000),
            )
            .with_long_prompt(
                100_000,
                token_prices(500_000, 50_000, 625_000, 2_500_000),
            ),
            ModelPrice::new(
                "gpt-6.1-sol",
                token_prices(2_000_000, 100_000, 2_500_000, 10_000_000),
            )
            .with_long_prompt(
                272_000,
                token_prices(4_000_000, 200_000, 5_000_000, 15_000_000),
            ),
        ])
    }

    #[test]
    fn a_request_just_over_the_prompt_limit_is_priced_as_a_long_prompt() {
        let cost = |model: &str, first: TokenCount, second: TokenCount| {
            let files = [FileSpend::new(
                None,
                vec![
                    CountedAnswer::new(at(NOW), Some(model), first),
                    CountedAnswer::new(at(NOW), Some(model), second),
                ],
            )];
            let spend = tally(files.iter(), &utc_now(), None);
            let total = MoneyTotal::of_models(
                &spend.this_month_by_model(),
                &long_prompt_prices(),
                &Registry::default(),
            );
            total.cost().micro_dollars()
        };

        // Haiku: 100,000 at $0.10; then 50,000 at $0.50 and 50,001 cached
        // at $0.05, per million.
        assert_eq!(
            cost(
                "claude-haiku-5-5",
                TokenCount::new(100_000, 0, 0),
                TokenCount::new(50_000, 50_001, 0)
            ),
            10_000 + 27_500
        );
        // GPT-6.1 Sol: 272,000 at $2; then 272,001 at $4, per million.
        assert_eq!(
            cost(
                "gpt-6.1-sol",
                TokenCount::new(272_000, 0, 0),
                TokenCount::new(272_001, 0, 0)
            ),
            544_000 + 1_088_004
        );
    }

    /// Made-up rows in the layout of the LiteLLM table (issue 23): the
    /// makers' own prices under plain keys, the prices of a reseller and
    /// of a cloud under their names. Input only, in dollars per token.
    const SERVED_TABLE: &str = r#"{
        "claude-x": {"input_cost_per_token": 3e-06, "output_cost_per_token": 0.0},
        "openrouter/anthropic/claude-x": {"input_cost_per_token": 4e-06, "output_cost_per_token": 0.0},
        "gpt-x": {"input_cost_per_token": 1e-06, "output_cost_per_token": 0.0},
        "azure/gpt-x": {"input_cost_per_token": 2e-06, "output_cost_per_token": 0.0},
        "gemini-x": {"input_cost_per_token": 5e-07, "output_cost_per_token": 0.0},
        "gemini/gemini-x": {"input_cost_per_token": 6e-07, "output_cost_per_token": 0.0}
    }"#;

    /// The providers OTTY has parts for in these tests: OpenRouter and
    /// OpenAI, each with the hosts of its servers.
    fn served_registry() -> Registry {
        let provider = |id: &str, hosts: [&'static str; 1]| {
            ProviderEntry::new(ProviderId::new(id), id, made_up_part(id))
                .with_hosts(hosts)
        };

        Registry::new(
            Vec::new(),
            vec![
                provider("openrouter", ["openrouter.ai"]),
                provider("openai", ["openai.com"]),
            ],
        )
    }

    /// What a million input tokens of `model` served by `provider` cost
    /// at the prices of [`SERVED_TABLE`], and how many requests had no
    /// price.
    fn served_cost(
        model: &str,
        provider: Option<ProviderRef>,
        registry: &Registry,
    ) -> (u64, u64) {
        let prices = read_litellm(SERVED_TABLE.as_bytes()).expect("the table");
        let tokens = million_in(Some(model), 1).with_provider(provider);

        let total = MoneyTotal::of_models(&[tokens], &prices, registry);

        (total.cost().micro_dollars(), total.unpriced_requests())
    }

    fn host(host: &str) -> Option<ProviderRef> {
        Some(ProviderRef::Host(host.to_string()))
    }

    fn id(id: &str) -> Option<ProviderRef> {
        Some(ProviderRef::of_id(id))
    }

    #[test]
    fn an_answer_served_by_openrouter_gets_openrouters_price() {
        let registry = served_registry();

        // OpenCode names the provider; Codex the host of its `base_url`.
        assert_eq!(
            served_cost("anthropic/claude-x", id("openrouter"), &registry),
            (4_000_000, 0)
        );
        assert_eq!(
            served_cost("anthropic/claude-x", host("openrouter.ai"), &registry),
            (4_000_000, 0)
        );
    }

    #[test]
    fn a_codex_answer_served_by_openai_gets_openais_price() {
        let registry = served_registry();

        assert_eq!(
            served_cost("gpt-x", id("openai"), &registry),
            (1_000_000, 0)
        );
        assert_eq!(
            served_cost("gpt-x", host("api.openai.com"), &registry),
            (1_000_000, 0),
            "not Azure's price"
        );
        assert_eq!(
            served_cost("gpt-x", id("azure"), &registry),
            (2_000_000, 0)
        );
    }

    #[test]
    fn an_answer_of_an_unknown_server_gets_the_makers_price() {
        let registry = served_registry();

        for provider in [host("llm.example.net"), id("example-cloud"), None] {
            assert_eq!(
                served_cost("claude-x", provider.clone(), &registry),
                (3_000_000, 0),
                "{provider:?}"
            );
        }
        assert_eq!(
            served_cost("claude-x", id("openrouter"), &registry),
            (3_000_000, 0),
            "OpenRouter has no row of the model as Anthropic names it"
        );
    }

    #[test]
    fn a_model_with_no_row_has_no_price() {
        let registry = served_registry();

        for provider in [id("openrouter"), host("llm.example.net"), None] {
            assert_eq!(
                served_cost("mystery-x", provider.clone(), &registry),
                (0, 1),
                "{provider:?}"
            );
        }
    }

    /// OpenCode writes `google`; the table names Google `gemini`. With no
    /// Google plugin the answer keeps its name and gets the maker's row;
    /// a part that serves `google` names the table's name.
    #[test]
    fn a_google_answer_is_priced_without_a_gemini_plugin() {
        let with_part = Registry::new(
            Vec::new(),
            vec![
                ProviderEntry::new(
                    ProviderId::new("google"),
                    "Google",
                    made_up_part("google"),
                )
                .with_hosts(["googleapis.com"])
                .with_priced_as("gemini"),
            ],
        );

        assert_eq!(
            served_cost("gemini-x", id("google"), &served_registry()),
            (500_000, 0)
        );
        assert_eq!(
            served_cost("gemini-x", id("google"), &with_part),
            (600_000, 0)
        );
    }
}
