//! OTTY's price list as an agent part asks it (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3): the part names a
//! model and who served it, and gets the model's API price from the list
//! prices OTTY keeps and renews (the LiteLLM price table), so it can price
//! the tokens of an answer whose cost the agent did not write. OTTY never
//! multiplies tokens by prices itself.

use std::sync::Arc;

use otty_plugin_host::agents::prices::{
    LongPromptPrice as PartLongPrompt, ModelPrice as PartPrice, PriceList,
    ProviderRef as PartProviderRef, TokenPrices as PartTokenPrices,
};

use crate::plugin_values::provider_of;
use crate::price_table::{ModelPrice, PriceTable, TokenPrices};
use crate::registry::Registry;

/// The list prices the agent parts price their answers at: the price
/// table, and the registry that names a provider to it. Two of them are
/// equal when both are, so a read of the tokens knows when the prices it
/// counted at changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiPrices {
    table: PriceTable,
    registry: Arc<Registry>,
}

impl ApiPrices {
    /// The prices of `table`, each provider named to it as `registry`
    /// names it ([`Registry::price_name_of`]). With an empty table no
    /// answer gets a price.
    ///
    /// ```
    /// use std::sync::Arc;
    ///
    /// use otty_agents::{ApiPrices, PriceTable, Registry};
    ///
    /// let prices = ApiPrices::new(
    ///     PriceTable::new(Vec::new()),
    ///     Arc::new(Registry::default()),
    /// );
    ///
    /// assert_eq!(prices, ApiPrices::default());
    /// ```
    pub fn new(table: PriceTable, registry: Arc<Registry>) -> Self {
        Self { table, registry }
    }
}

impl PriceList for ApiPrices {
    /// The price of `model` served by `provider`: the row of who served
    /// it, named as the registry names the provider to the price table,
    /// else the maker's row ([`PriceTable::price_of`]).
    fn price_of(
        &self,
        model: &str,
        provider: Option<PartProviderRef>,
    ) -> Option<PartPrice> {
        let provider = provider.as_ref().map(provider_of);
        let served_by = provider
            .as_ref()
            .and_then(|provider| self.registry.price_name_of(provider));

        let price = self.table.price_of(model, served_by)?;

        Some(part_price(price))
    }
}

/// `price` as the agent contract gives it, in millionths of a US dollar
/// per million tokens.
fn part_price(price: &ModelPrice) -> PartPrice {
    PartPrice {
        standard: part_token_prices(price.standard()),
        long_prompt: price.long_prompt().map(|long| PartLongPrompt {
            over_tokens: long.over_tokens(),
            prices: part_token_prices(long.prices()),
        }),
    }
}

fn part_token_prices(prices: &TokenPrices) -> PartTokenPrices {
    PartTokenPrices {
        input: prices.input.micro_dollars(),
        cached: prices.cached.micro_dollars(),
        cache_written: prices.cache_written.micro_dollars(),
        output: prices.output.micro_dollars(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::litellm::read_litellm;
    use crate::provider::{ProviderEntry, ProviderId};
    use crate::testing::{made_up_part, token_prices};

    /// Made-up prices of one model by its maker and by a cloud the price
    /// table names `example_cloud`, with a long prompt price at the
    /// cloud.
    fn table() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new("m", token_prices(1_000_000, 0, 0, 0)),
            ModelPrice::new("m", token_prices(2_000_000, 10, 20, 30))
                .with_provider("example_cloud")
                .with_long_prompt(200_000, token_prices(4_000_000, 1, 2, 3)),
        ])
    }

    /// The registry of a cloud whose part serves `example.com` and names
    /// it `example_cloud` to the price table.
    fn registry() -> Arc<Registry> {
        let cloud = ProviderEntry::new(
            ProviderId::new("example-cloud"),
            "Example Cloud",
            made_up_part("example-cloud"),
        )
        .with_hosts(["example.com"])
        .with_priced_as("example_cloud");

        Arc::new(Registry::new(Vec::new(), vec![cloud]))
    }

    fn input_of(price: Option<PartPrice>) -> Option<u64> {
        price.map(|price| price.standard.input)
    }

    /// Decision 3: a part asks for the price of the model who served the
    /// answer, by the provider's id or the host of its server.
    #[test]
    fn the_price_of_who_served_the_answer_is_given() {
        let prices = ApiPrices::new(table(), registry());
        let by_id = PartProviderRef::Id("example-cloud".to_string());
        let by_host = PartProviderRef::Host("api.example.com".to_string());

        let price = prices.price_of("m", Some(by_id));
        let from_host = prices.price_of("m", Some(by_host));

        let Some(price) = price else {
            panic!("the cloud's price");
        };
        assert_eq!(price.standard.input, 2_000_000);
        assert_eq!(price.standard.output, 30);
        let long = price.long_prompt.expect("a long prompt price");
        assert_eq!((long.over_tokens, long.prices.input), (200_000, 4_000_000));
        assert_eq!(input_of(from_host), Some(2_000_000));
    }

    #[test]
    fn with_no_price_of_who_served_it_the_makers_price_is_given() {
        let prices = ApiPrices::new(table(), registry());
        let unknown = PartProviderRef::Id("other".to_string());

        assert_eq!(input_of(prices.price_of("m", None)), Some(1_000_000));
        assert_eq!(
            input_of(prices.price_of("m", Some(unknown))),
            Some(1_000_000)
        );
    }

    #[test]
    fn a_model_the_table_has_no_price_for_gets_none() {
        let prices = ApiPrices::new(table(), registry());

        assert!(prices.price_of("mystery", None).is_none());
        assert!(ApiPrices::default().price_of("m", None).is_none());
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

    /// The input price of `model` served by `provider` at the prices of
    /// [`SERVED_TABLE`], per million tokens.
    fn served_input(
        model: &str,
        provider: Option<PartProviderRef>,
        registry: Registry,
    ) -> Option<u64> {
        let table = read_litellm(SERVED_TABLE.as_bytes()).expect("the table");
        let prices = ApiPrices::new(table, Arc::new(registry));

        input_of(prices.price_of(model, provider))
    }

    fn host(host: &str) -> Option<PartProviderRef> {
        Some(PartProviderRef::Host(host.to_string()))
    }

    fn id(id: &str) -> Option<PartProviderRef> {
        Some(PartProviderRef::Id(id.to_string()))
    }

    #[test]
    fn an_answer_served_by_openrouter_gets_openrouters_price() {
        // OpenCode names the provider; Codex the host of its `base_url`.
        assert_eq!(
            served_input(
                "anthropic/claude-x",
                id("openrouter"),
                served_registry()
            ),
            Some(4_000_000)
        );
        assert_eq!(
            served_input(
                "anthropic/claude-x",
                host("openrouter.ai"),
                served_registry()
            ),
            Some(4_000_000)
        );
    }

    #[test]
    fn a_codex_answer_served_by_openai_gets_openais_price() {
        assert_eq!(
            served_input("gpt-x", id("openai"), served_registry()),
            Some(1_000_000)
        );
        assert_eq!(
            served_input("gpt-x", host("api.openai.com"), served_registry()),
            Some(1_000_000),
            "not Azure's price"
        );
        assert_eq!(
            served_input("gpt-x", id("azure"), served_registry()),
            Some(2_000_000)
        );
    }

    #[test]
    fn an_answer_of_an_unknown_server_gets_the_makers_price() {
        for provider in [host("llm.example.net"), id("example-cloud"), None] {
            assert_eq!(
                served_input("claude-x", provider.clone(), served_registry()),
                Some(3_000_000),
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
            served_input("gemini-x", id("google"), served_registry()),
            Some(500_000)
        );
        assert_eq!(
            served_input("gemini-x", id("google"), with_part),
            Some(600_000)
        );
    }
}
