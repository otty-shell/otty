//! List prices of models per million tokens. OTTY gives them to the agent
//! parts, which price the tokens of their answers themselves (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3).

use std::collections::HashMap;

use crate::dollars::Dollars;

/// List prices of models: the price of each model under its plain name,
/// the maker's own as a rule, and the prices of named providers, such as
/// clouds, resellers and the makers the table names only with a prefix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PriceTable {
    makers: HashMap<String, ModelPrice>,
    providers: HashMap<String, HashMap<String, ModelPrice>>,
}

impl PriceTable {
    /// A table of the prices of `models`. Of two prices of the same model
    /// id and provider, the first is kept.
    ///
    /// ```
    /// use otty_agents::{Dollars, ModelPrice, PriceTable, TokenPrices};
    ///
    /// let prices = TokenPrices {
    ///     input: Dollars::from_micro_dollars(1_250_000),
    ///     ..TokenPrices::default()
    /// };
    /// let table = PriceTable::new(vec![ModelPrice::new("gpt-5", prices)]);
    ///
    /// assert!(table.price_of("gpt-5", None).is_some());
    /// ```
    pub fn new(models: Vec<ModelPrice>) -> Self {
        let mut table = Self::default();
        for price in models {
            let rows = match &price.provider {
                Some(provider) => {
                    table.providers.entry(provider.clone()).or_default()
                },
                None => &mut table.makers,
            };
            rows.entry(price.model.clone()).or_insert(price);
        }

        table
    }

    /// The price of the model named `model` as the agent wrote it, served
    /// by the provider the table names `served_by`, such as `openrouter`:
    ///
    /// 1. the row of that provider and the model;
    /// 2. else the row of the plain model name, the maker's own price;
    /// 3. else `None`: a similar model is never used instead.
    ///
    /// Each step takes the row of the same id, else the row of the model a
    /// dated snapshot belongs to (`claude-opus-4-5-20251101` is priced as
    /// `claude-opus-4-5`, `gpt-5-2025-08-07` as `gpt-5`).
    pub fn price_of(
        &self,
        model: &str,
        served_by: Option<&str>,
    ) -> Option<&ModelPrice> {
        let provider_rows =
            served_by.and_then(|provider| self.providers.get(provider));
        if let Some(price) = provider_rows.and_then(|rows| row_of(rows, model))
        {
            return Some(price);
        }

        row_of(&self.makers, model)
    }
}

#[cfg(test)]
impl PriceTable {
    /// How many prices the table has, of makers and providers.
    pub(crate) fn len(&self) -> usize {
        self.makers.len()
            + self.providers.values().map(HashMap::len).sum::<usize>()
    }
}

/// The list prices of one model per million tokens: one set for every
/// prompt, or, for a model that costs more for a long prompt, a second set
/// for the requests whose prompt is over a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPrice {
    model: String,
    provider: Option<String>,
    standard: TokenPrices,
    long_prompt: Option<LongPromptPrice>,
}

impl ModelPrice {
    /// The model id, such as `claude-opus-5-5` or `gpt-6.1-sol`.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The provider whose price this is, by the name the price table
    /// gives it, such as `openrouter`; `None` for the maker's own price.
    pub fn provider(&self) -> Option<&str> {
        self.provider.as_deref()
    }

    /// The prices of a request whose prompt is not long.
    pub(crate) fn standard(&self) -> &TokenPrices {
        &self.standard
    }

    /// The prices of a request with a long prompt; `None` when the model
    /// has one price for every prompt.
    pub(crate) fn long_prompt(&self) -> Option<&LongPromptPrice> {
        self.long_prompt.as_ref()
    }

    /// The prices of `model` per million tokens, the same for every
    /// prompt.
    ///
    /// ```
    /// use otty_agents::{Dollars, ModelPrice, TokenPrices};
    ///
    /// let price = ModelPrice::new(
    ///     "gpt-5",
    ///     TokenPrices {
    ///         input: Dollars::from_micro_dollars(1_250_000),
    ///         cached: Dollars::from_micro_dollars(125_000),
    ///         cache_written: Dollars::from_micro_dollars(1_250_000),
    ///         output: Dollars::from_micro_dollars(10_000_000),
    ///     },
    /// );
    ///
    /// assert_eq!(price.model(), "gpt-5");
    /// ```
    pub fn new(model: &str, prices: TokenPrices) -> Self {
        Self {
            model: model.to_string(),
            provider: None,
            standard: prices,
            long_prompt: None,
        }
    }

    /// The same prices as the price of `provider`, named as the price
    /// table names it, such as `openrouter` or `xai`: they price only the
    /// answers that provider served.
    ///
    /// ```
    /// use otty_agents::{ModelPrice, TokenPrices};
    ///
    /// let price = ModelPrice::new("anthropic/claude-x", TokenPrices::default())
    ///     .with_provider("openrouter");
    ///
    /// assert_eq!(price.provider(), Some("openrouter"));
    /// ```
    pub fn with_provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
        self
    }

    /// The same prices, with other `prices` per million tokens for a
    /// request whose prompt is over `over_tokens` tokens.
    ///
    /// ```
    /// use otty_agents::{Dollars, ModelPrice, TokenPrices};
    ///
    /// let per_million = |micro_dollars| TokenPrices {
    ///     input: Dollars::from_micro_dollars(micro_dollars),
    ///     ..TokenPrices::default()
    /// };
    ///
    /// let price = ModelPrice::new("claude-haiku-5-5", per_million(100_000))
    ///     .with_long_prompt(100_000, per_million(500_000));
    ///
    /// assert_eq!(price.model(), "claude-haiku-5-5");
    /// ```
    pub fn with_long_prompt(
        self,
        over_tokens: u64,
        prices: TokenPrices,
    ) -> Self {
        Self {
            long_prompt: Some(LongPromptPrice {
                over_tokens,
                prices,
            }),
            ..self
        }
    }
}

/// The prices of a model for a request with a long prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LongPromptPrice {
    over_tokens: u64,
    prices: TokenPrices,
}

impl LongPromptPrice {
    /// The prompt is long when it has more tokens than this: its input
    /// with the part read from or written to the cache.
    pub(crate) fn over_tokens(&self) -> u64 {
        self.over_tokens
    }

    /// The prices of a request with a long prompt.
    pub(crate) fn prices(&self) -> &TokenPrices {
        &self.prices
    }
}

/// Prices per million tokens of input, cached input, input written to the
/// cache and output.
///
/// A plain set of four prices with nothing to keep true between them, so
/// its fields are open: each price is named where it is written, and two
/// of them can not be swapped by mistake.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenPrices {
    /// The price of a million input tokens not read from the cache.
    pub input: Dollars,
    /// The price of a million input tokens read from the cache.
    pub cached: Dollars,
    /// The price of a million input tokens written to the cache.
    pub cache_written: Dollars,
    /// The price of a million output tokens.
    pub output: Dollars,
}

/// The price of `model` among `rows`: the row of the same id, else the row
/// of the model a dated snapshot belongs to.
fn row_of<'a>(
    rows: &'a HashMap<String, ModelPrice>,
    model: &str,
) -> Option<&'a ModelPrice> {
    if let Some(price) = rows.get(model) {
        return Some(price);
    }

    let base = snapshot_base(model)?;
    rows.get(base)
}

/// The model a dated snapshot `model` belongs to: `claude-opus-4-5` of
/// `claude-opus-4-5-20251101` as Anthropic writes the date, `gpt-5` of
/// `gpt-5-2025-08-07` as OpenAI does. `None` when `model` does not end with
/// such a date.
fn snapshot_base(model: &str) -> Option<&str> {
    for date_length in [8, 10] {
        let Some(split) = model.len().checked_sub(date_length + 1) else {
            continue;
        };
        let (Some(base), Some(rest)) = (model.get(..split), model.get(split..))
        else {
            continue;
        };

        let is_dated = rest.strip_prefix('-').is_some_and(is_snapshot_date);
        if is_dated && !base.is_empty() {
            return Some(base);
        }
    }

    None
}

/// Whether `text` is the date of a model snapshot: `20251101` as Anthropic
/// writes it, or `2025-08-07` as OpenAI does.
fn is_snapshot_date(text: &str) -> bool {
    let bytes = text.as_bytes();

    match bytes.len() {
        8 => bytes.iter().all(u8::is_ascii_digit),
        10 => bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::token_prices;

    /// Made-up prices, in micro-dollars per million tokens.
    fn table() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new(
                "gpt-5",
                token_prices(1_000_000, 100_000, 1_000_000, 8_000_000),
            ),
            ModelPrice::new(
                "claude-opus-4",
                token_prices(15_000_000, 1_500_000, 18_750_000, 75_000_000),
            ),
            ModelPrice::new(
                "claude-opus-4-5",
                token_prices(5_000_000, 500_000, 6_250_000, 25_000_000),
            ),
        ])
    }

    fn model_of<'a>(table: &'a PriceTable, model: &str) -> Option<&'a str> {
        table.price_of(model, None).map(ModelPrice::model)
    }

    #[test]
    fn a_known_model_is_priced_by_its_id() {
        let table = table();

        let price = table.price_of("gpt-5", None);

        assert_eq!(price.map(ModelPrice::model), Some("gpt-5"));
        assert_eq!(
            model_of(&table, "claude-opus-4-5"),
            Some("claude-opus-4-5")
        );
    }

    #[test]
    fn a_dated_snapshot_is_priced_as_its_model() {
        let table = table();

        assert_eq!(
            model_of(&table, "claude-opus-4-5-20251101"),
            Some("claude-opus-4-5")
        );
        assert_eq!(
            model_of(&table, "claude-opus-4-20250514"),
            Some("claude-opus-4")
        );
        assert_eq!(model_of(&table, "gpt-5-2025-08-07"), Some("gpt-5"));
    }

    #[test]
    fn a_similar_model_is_not_priced() {
        let table = table();

        for model in [
            "gpt-5-mini",
            "gpt-5.1",
            "gpt-5-2025",
            "gpt-5-2025-08-7",
            "claude-opus-4-6",
            "claude-opus-4-5-latest",
            "claude-opus-4-5-2025110",
            "gpt",
            "",
        ] {
            assert_eq!(model_of(&table, model), None, "{model}");
        }
    }

    #[test]
    fn the_row_of_the_same_id_wins_over_the_model_of_a_snapshot() {
        let table = PriceTable::new(vec![
            ModelPrice::new(
                "claude-opus-4-5",
                token_prices(5_000_000, 0, 0, 0),
            ),
            ModelPrice::new(
                "claude-opus-4-5-20251101",
                token_prices(4_000_000, 0, 0, 0),
            ),
        ]);

        assert_eq!(
            model_of(&table, "claude-opus-4-5-20251101"),
            Some("claude-opus-4-5-20251101")
        );
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn of_two_prices_of_one_model_the_first_is_kept() {
        let table = PriceTable::new(vec![
            ModelPrice::new("gpt-5", token_prices(1_250_000, 0, 0, 0)),
            ModelPrice::new("gpt-5", token_prices(9_000_000, 0, 0, 0)),
        ]);

        let input = table
            .price_of("gpt-5", None)
            .map(|price| price.standard().input.micro_dollars());

        assert_eq!(input, Some(1_250_000));
        assert_eq!(table.len(), 1);
    }

    /// The input price of `model` served by `served_by`, in micro-dollars
    /// per million tokens.
    fn input_of(
        table: &PriceTable,
        model: &str,
        served_by: Option<&str>,
    ) -> Option<u64> {
        table
            .price_of(model, served_by)
            .map(|price| price.standard().input.micro_dollars())
    }

    /// Made-up prices of one model sold by its maker and by two
    /// resellers, and of a model only a reseller sells.
    fn table_of_resellers() -> PriceTable {
        PriceTable::new(vec![
            ModelPrice::new(
                "claude-opus-4-5",
                token_prices(5_000_000, 0, 0, 0),
            ),
            ModelPrice::new(
                "claude-opus-4-5",
                token_prices(6_000_000, 0, 0, 0),
            )
            .with_provider("example-cloud"),
            ModelPrice::new(
                "anthropic/claude-opus-4-5",
                token_prices(7_000_000, 0, 0, 0),
            )
            .with_provider("example-router"),
            ModelPrice::new("grok-9", token_prices(3_000_000, 0, 0, 0))
                .with_provider("example-maker"),
        ])
    }

    #[test]
    fn the_row_of_who_served_the_answer_prices_it() {
        let table = table_of_resellers();

        assert_eq!(
            input_of(&table, "claude-opus-4-5", Some("example-cloud")),
            Some(6_000_000)
        );
        assert_eq!(
            input_of(
                &table,
                "anthropic/claude-opus-4-5",
                Some("example-router")
            ),
            Some(7_000_000)
        );
        assert_eq!(
            input_of(&table, "grok-9", Some("example-maker")),
            Some(3_000_000)
        );
        assert_eq!(
            input_of(&table, "claude-opus-4-5-20251101", Some("example-cloud")),
            Some(6_000_000),
            "a dated snapshot finds the row of who served it"
        );
    }

    #[test]
    fn with_no_row_of_who_served_it_an_answer_gets_the_makers_price() {
        let table = table_of_resellers();

        for served_by in [None, Some("example-other"), Some("example-router")] {
            assert_eq!(
                input_of(&table, "claude-opus-4-5", served_by),
                Some(5_000_000),
                "{served_by:?}"
            );
        }
    }

    #[test]
    fn the_row_of_a_provider_never_prices_what_another_served() {
        let table = table_of_resellers();

        assert_eq!(input_of(&table, "grok-9", None), None);
        assert_eq!(input_of(&table, "grok-9", Some("example-cloud")), None);
        assert_eq!(input_of(&table, "anthropic/claude-opus-4-5", None), None);
        assert_eq!(table.len(), 4);
    }
}
