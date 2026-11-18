//! List prices of models per million tokens, and what a number of tokens
//! costs at them.

use std::collections::HashMap;

use crate::dollars::Dollars;
use crate::spend::ModelTokens;
use crate::token_count::TokenCount;

/// How many tokens a price in the table is for.
const TOKENS_PER_PRICE: u128 = 1_000_000;

/// List prices of models, by model id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PriceTable {
    models: HashMap<String, ModelPrice>,
}

impl PriceTable {
    /// A table of the prices of `models`. Of two prices of the same model
    /// id, the first is kept.
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
    /// assert!(table.price_of("gpt-5").is_some());
    /// ```
    pub fn new(models: Vec<ModelPrice>) -> Self {
        let mut by_id = HashMap::with_capacity(models.len());
        for price in models {
            by_id.entry(price.model.clone()).or_insert(price);
        }

        Self { models: by_id }
    }

    /// The price of the model named `model` as the agent wrote it: the row
    /// of the same id, else the row of the model a dated snapshot belongs
    /// to (`claude-opus-4-5-20251101` is priced as `claude-opus-4-5`,
    /// `gpt-5-2025-08-07` as `gpt-5`). `None` when the table has no price
    /// for it; a similar model is never used instead.
    pub fn price_of(&self, model: &str) -> Option<&ModelPrice> {
        if let Some(price) = self.models.get(model) {
            return Some(price);
        }

        let base = snapshot_base(model)?;
        self.models.get(base)
    }
}

#[cfg(test)]
impl PriceTable {
    /// How many models have a price.
    pub(crate) fn len(&self) -> usize {
        self.models.len()
    }
}

/// The list prices of one model per million tokens: one set for every
/// prompt, or, for a model that costs more for a long prompt, a second set
/// for the requests whose prompt is over a limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPrice {
    model: String,
    standard: TokenPrices,
    long_prompt: Option<LongPromptPrice>,
}

impl ModelPrice {
    /// The model id, such as `claude-opus-5-5` or `gpt-6.1-sol`.
    pub fn model(&self) -> &str {
        &self.model
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
            standard: prices,
            long_prompt: None,
        }
    }

    /// The same prices, with other `prices` per million tokens for a
    /// request whose prompt is over `over_tokens` tokens.
    ///
    /// The token counter counts the tokens of long prompts only over the
    /// limits the LiteLLM price table is known to use; with another
    /// `over_tokens`, every request is priced at the standard prices.
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

    /// What the tokens of `model` cost at these prices: the tokens of its
    /// long prompts at the long prompt prices, the rest at the standard
    /// ones. When the tokens were not counted over the limit of the long
    /// prompt price, all of them are priced at the standard prices.
    pub fn cost(&self, model: &ModelTokens) -> Dollars {
        let long = self.long_prompt.as_ref().and_then(|long_prompt| {
            let tokens = model.long_prompts_over(long_prompt.over_tokens)?;
            Some((long_prompt, tokens))
        });

        match long {
            Some((long_prompt, long_tokens)) => {
                let standard = model.tokens() - long_tokens;

                self.standard.cost(standard)
                    + long_prompt.prices.cost(long_tokens)
            },
            None => self.standard.cost(model.tokens()),
        }
    }
}

#[cfg(test)]
impl ModelPrice {
    /// The prices of a request whose prompt is not long.
    pub(crate) fn standard(&self) -> &TokenPrices {
        &self.standard
    }

    /// The prices of a request with a long prompt; `None` when the model
    /// has one price for every prompt.
    pub(crate) fn long_prompt(&self) -> Option<&LongPromptPrice> {
        self.long_prompt.as_ref()
    }
}

/// The prices of a model for a request with a long prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LongPromptPrice {
    over_tokens: u64,
    prices: TokenPrices,
}

#[cfg(test)]
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

impl TokenPrices {
    /// What `tokens` cost at these prices, to the nearest micro-dollar.
    /// The input written to the cache is priced as a cache write, the rest
    /// of the input as input.
    pub fn cost(&self, tokens: TokenCount) -> Dollars {
        let written = tokens.cache_written().min(tokens.input());
        let not_written = tokens.input() - written;

        let per_million_tokens = price_of_count(not_written, self.input)
            + price_of_count(written, self.cache_written)
            + price_of_count(tokens.cached(), self.cached)
            + price_of_count(tokens.output(), self.output);
        let micro_dollars =
            (per_million_tokens + TOKENS_PER_PRICE / 2) / TOKENS_PER_PRICE;

        Dollars::from_micro_dollars(
            u64::try_from(micro_dollars).unwrap_or(u64::MAX),
        )
    }
}

/// `count` tokens at `price` per million, in millionths of a
/// micro-dollar. It is wide enough that no count and price overflow it.
fn price_of_count(count: u64, price: Dollars) -> u128 {
    u128::from(count) * u128::from(price.micro_dollars())
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
        table.price_of(model).map(ModelPrice::model)
    }

    #[test]
    fn a_known_model_is_priced_by_its_id() {
        let table = table();

        let price = table.price_of("gpt-5");

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
    fn input_cache_reads_cache_writes_and_output_are_priced_apart() {
        let price = ModelPrice::new(
            "m",
            token_prices(3_000_000, 300_000, 3_750_000, 15_000_000),
        );
        // 1000 tokens of input, 400 of them written to the cache; 2000
        // read from the cache; 100 of output.
        let tokens = TokenCount::new(1000, 2000, 100).with_cache_written(400);

        let cost = price.standard().cost(tokens);

        // 600 × $3 + 400 × $3.75 + 2000 × $0.30 + 100 × $15, per million.
        assert_eq!(cost, Dollars::from_micro_dollars(1800 + 1500 + 600 + 1500));
    }

    #[test]
    fn a_cost_is_rounded_to_the_nearest_micro_dollar() {
        let price = ModelPrice::new("m", token_prices(400_000, 0, 0, 0));

        let cost = |input| price.standard().cost(TokenCount::new(input, 0, 0));

        assert_eq!(cost(1).micro_dollars(), 0);
        assert_eq!(cost(2).micro_dollars(), 1);
        assert_eq!(cost(5).micro_dollars(), 2);
    }

    #[test]
    fn a_cost_too_large_to_keep_stays_at_the_largest_amount() {
        let price =
            ModelPrice::new("m", token_prices(75_000_000, 0, 0, 75_000_000));
        let tokens = TokenCount::new(u64::MAX, 0, u64::MAX);

        assert_eq!(price.standard().cost(tokens).micro_dollars(), u64::MAX);
    }

    /// 3 million input tokens in 3 requests, one of them with a long
    /// prompt of a million tokens, over 100,000 tokens and over 272,000.
    fn three_requests() -> ModelTokens {
        let long = TokenCount::new(1_000_000, 0, 0);

        ModelTokens::new(Some("m"), TokenCount::new(3_000_000, 0, 0), 3)
            .with_long_prompts(100_000, long)
            .with_long_prompts(272_000, long)
    }

    #[test]
    fn long_prompts_are_priced_at_the_long_prompt_price() {
        let price = ModelPrice::new(
            "m",
            token_prices(1_000_000, 100_000, 1_250_000, 5_000_000),
        )
        .with_long_prompt(
            272_000,
            token_prices(5_000_000, 500_000, 6_250_000, 20_000_000),
        );

        let cost = price.cost(&three_requests());

        // 2 million at $1 and a million at $5.
        assert_eq!(cost, Dollars::from_micro_dollars(7_000_000));
        assert_eq!(
            price.long_prompt().map(LongPromptPrice::over_tokens),
            Some(272_000)
        );
    }

    #[test]
    fn a_model_with_one_price_prices_long_prompts_the_same() {
        let price = ModelPrice::new(
            "m",
            token_prices(1_000_000, 100_000, 1_250_000, 5_000_000),
        );

        let cost = price.cost(&three_requests());

        assert_eq!(cost, Dollars::from_micro_dollars(3_000_000));
        assert_eq!(price.long_prompt(), None);
    }

    #[test]
    fn a_long_prompt_limit_the_tokens_are_not_counted_over_is_not_used() {
        let price = ModelPrice::new("m", token_prices(1_000_000, 0, 0, 0))
            .with_long_prompt(150_000, token_prices(5_000_000, 0, 0, 0));

        let cost = price.cost(&three_requests());

        assert_eq!(cost, Dollars::from_micro_dollars(3_000_000));
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
            .price_of("gpt-5")
            .map(|price| price.standard().input.micro_dollars());

        assert_eq!(input, Some(1_250_000));
        assert_eq!(table.len(), 1);
    }
}
