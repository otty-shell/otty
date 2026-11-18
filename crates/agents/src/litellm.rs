//! Reads the LiteLLM price table into a [`PriceTable`] (spec Decision 16).
//!
//! The table is the JSON file `model_prices_and_context_window.json` of the
//! open source LiteLLM project, <https://github.com/BerriAI/litellm>. It is
//! an object with one entry per model key. An entry gives its prices in
//! dollars per token: `input_cost_per_token`, `output_cost_per_token`,
//! `cache_read_input_token_cost` and `cache_creation_input_token_cost`, and,
//! for a model that costs more for a long prompt, the same names with
//! `_above_<N>k_tokens` at the end. `litellm_provider` says who sells the
//! model at these prices.
//!
//! The token counter knows the `<N>` the file uses: it keeps the tokens of
//! long prompts only over the limits of `PROMPT_LIMITS` in `spend.rs`. A
//! model priced over another limit is priced at its standard prices for
//! every request, so a new limit in the file must be added there too.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::dollars::Dollars;
use crate::error::SourceError;
use crate::price_table::{ModelPrice, PriceTable, TokenPrices};

/// The `litellm_provider` of the makers that sell their own models, whose
/// prices are used. Clouds and resellers, such as `bedrock`, `vertex_ai-*`,
/// `azure` or `openrouter`, are not here.
const MODEL_MAKERS: [&str; 10] = [
    "anthropic",
    "openai",
    "deepseek",
    "gemini",
    "xai",
    "mistral",
    "moonshot",
    "zai",
    "minimax",
    "dashscope",
];
/// Micro-dollars per million tokens in one dollar per token.
const MICRO_DOLLARS_PER_MILLION: f64 = 1e12;

/// Reads the LiteLLM price table `json`.
///
/// A key is used when the model is sold by its maker, as the agents use
/// it: a plain key such as `claude-sonnet-4-5`, or one written as
/// `<provider>/<model>`, such as `deepseek/deepseek-v3.2`, whose provider is
/// the maker. The plain key wins when both are there. The keys of clouds
/// and resellers (Bedrock, Vertex, Azure, OpenRouter and the like) are
/// left out, so their prices never mix in. An entry with no input or no
/// output price is left out.
///
/// A missing cache price is the input price of the same prompt length: no
/// discount is guessed. A cache write never costs less than plain input:
/// the file writes 0 for the providers that charge a write as plain input,
/// such as DeepSeek. A model with long prompt prices gets those of its
/// lowest limit; a missing long output price is the standard one.
///
/// The file is not what the code expects when it is not a JSON object or
/// no model in it has a price.
pub(crate) fn read_litellm(json: &[u8]) -> Result<PriceTable, SourceError> {
    let entries: BTreeMap<String, Value> = serde_json::from_slice(json)
        .map_err(|_| unexpected("the price file is not a JSON object"))?;

    let mut plain = Vec::new();
    let mut with_provider = Vec::new();
    for (key, entry) in &entries {
        let Some((model, is_plain)) = maker_model(key, entry) else {
            continue;
        };
        let Some(price) = model_price(model, entry) else {
            continue;
        };

        if is_plain {
            plain.push(price);
        } else {
            with_provider.push(price);
        }
    }

    if plain.is_empty() && with_provider.is_empty() {
        return Err(unexpected("the price file has no prices"));
    }

    plain.extend(with_provider);
    Ok(PriceTable::new(plain))
}

/// The model the entry of `key` prices, and whether `key` is plain; `None`
/// when the entry is not sold by a maker of [`MODEL_MAKERS`] or its key is
/// not the plain model or `<provider>/<model>`.
fn maker_model<'a>(key: &'a str, entry: &Value) -> Option<(&'a str, bool)> {
    let provider = entry.get("litellm_provider")?.as_str()?;
    if !MODEL_MAKERS.contains(&provider) {
        return None;
    }

    if !key.contains('/') {
        return Some((key, true));
    }

    let model = key.strip_prefix(provider)?.strip_prefix('/')?;
    (!model.is_empty() && !model.contains('/')).then_some((model, false))
}

/// The prices of `model` in `entry`; `None` without an input or an output
/// price.
fn model_price(model: &str, entry: &Value) -> Option<ModelPrice> {
    let input = price_field(entry, "input_cost_per_token")?;
    let output = price_field(entry, "output_cost_per_token")?;
    let cached = price_field(entry, "cache_read_input_token_cost");
    let written = price_field(entry, "cache_creation_input_token_cost");

    let price = ModelPrice::new(
        model,
        TokenPrices {
            input,
            cached: cached.unwrap_or(input),
            cache_written: written.unwrap_or(input).max(input),
            output,
        },
    );

    let Some(limit) = long_prompt_limit(entry) else {
        return Some(price);
    };
    let above = format!("_above_{}k_tokens", limit / 1000);
    let Some(long_input) =
        price_field(entry, &format!("input_cost_per_token{above}"))
    else {
        return Some(price);
    };
    let long_output =
        price_field(entry, &format!("output_cost_per_token{above}"));
    let long_cached =
        price_field(entry, &format!("cache_read_input_token_cost{above}"));
    let long_written =
        price_field(entry, &format!("cache_creation_input_token_cost{above}"));

    Some(price.with_long_prompt(
        limit,
        TokenPrices {
            input: long_input,
            cached: long_cached.unwrap_or(long_input),
            cache_written: long_written.unwrap_or(long_input).max(long_input),
            output: long_output.unwrap_or(output),
        },
    ))
}

/// The lowest prompt limit, in tokens, above which `entry` has another
/// input price: 272,000 for `input_cost_per_token_above_272k_tokens`.
/// The prices of the batch, flex or priority tiers are not looked at.
///
/// The limit must be one of `PROMPT_LIMITS` in `spend.rs`, the limits the
/// token counter counts long prompts over; see the module doc.
fn long_prompt_limit(entry: &Value) -> Option<u64> {
    entry
        .as_object()?
        .keys()
        .filter_map(|field| {
            let thousands = field
                .strip_prefix("input_cost_per_token_above_")?
                .strip_suffix("k_tokens")?;
            thousands.parse::<u64>().ok()?.checked_mul(1000)
        })
        .min()
}

/// The price `field` of `entry` per million tokens; `None` when it is
/// missing or not a price.
fn price_field(entry: &Value, field: &str) -> Option<Dollars> {
    entry
        .get(field)?
        .as_f64()
        .and_then(micro_dollars_per_million)
        .map(Dollars::from_micro_dollars)
}

/// A price in dollars per token as micro-dollars per million tokens,
/// rounded to the nearest one: `3e-06` is `3_000_000`. `None` for a price
/// below zero or too large to keep.
fn micro_dollars_per_million(dollars_per_token: f64) -> Option<u64> {
    let rounded = (dollars_per_token * MICRO_DOLLARS_PER_MILLION).round();

    (0.0..u64::MAX as f64)
        .contains(&rounded)
        .then_some(rounded as u64)
}

/// The failure of a file that is not what the code expects.
fn unexpected(reason: &str) -> SourceError {
    SourceError::UnexpectedAnswer(reason.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::price_table::ModelPrice;

    /// A small made-up table in the layout of the LiteLLM file: the field
    /// names are the real ones, the models and prices are made up.
    const TABLE: &str = r#"{
        "sample_spec": {
            "input_cost_per_token": 0.0,
            "litellm_provider": "one of https://docs.litellm.ai/docs/providers",
            "output_cost_per_token": 0.0
        },
        "claude-sonnet-4-5": {
            "cache_creation_input_token_cost": 3.75e-06,
            "cache_read_input_token_cost": 3e-07,
            "input_cost_per_token": 3e-06,
            "litellm_provider": "anthropic",
            "mode": "chat",
            "output_cost_per_token": 1.5e-05
        },
        "claude-haiku-5-5": {
            "cache_creation_input_token_cost": 1.25e-07,
            "cache_creation_input_token_cost_above_100k_tokens": 6.25e-07,
            "cache_creation_input_token_cost_above_1hr": 2e-07,
            "cache_read_input_token_cost": 1e-08,
            "cache_read_input_token_cost_above_100k_tokens": 5e-08,
            "input_cost_per_token": 1e-07,
            "input_cost_per_token_above_100k_tokens": 5e-07,
            "litellm_provider": "anthropic",
            "output_cost_per_token": 5e-07,
            "output_cost_per_token_above_100k_tokens": 2.5e-06
        },
        "gpt-5.5": {
            "cache_read_input_token_cost": 5e-07,
            "cache_read_input_token_cost_above_272k_tokens": 1e-06,
            "cache_read_input_token_cost_priority": 1e-06,
            "input_cost_per_token": 5e-06,
            "input_cost_per_token_above_272k_tokens": 1e-05,
            "input_cost_per_token_above_272k_tokens_priority": 2e-05,
            "litellm_provider": "openai",
            "output_cost_per_token": 3e-05,
            "output_cost_per_token_above_272k_tokens": 4.5e-05
        },
        "gpt-5-pro": {
            "input_cost_per_token": 1.5e-05,
            "litellm_provider": "openai",
            "output_cost_per_token": 0.00012
        },
        "gemini-pro-long": {
            "input_cost_per_token": 1e-06,
            "input_cost_per_token_above_128k_tokens": 2e-06,
            "input_cost_per_token_above_256k_tokens": 4e-06,
            "litellm_provider": "gemini",
            "output_cost_per_token": 4e-06
        },
        "deepseek-chat": {
            "cache_read_input_token_cost": 2.8e-08,
            "input_cost_per_token": 2.8e-07,
            "litellm_provider": "deepseek",
            "output_cost_per_token": 4.2e-07
        },
        "deepseek/deepseek-chat": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "deepseek",
            "output_cost_per_token": 9e-06
        },
        "deepseek/deepseek-v3.2": {
            "cache_creation_input_token_cost": 0.0,
            "cache_read_input_token_cost": 2.8e-08,
            "input_cost_per_token": 2.8e-07,
            "litellm_provider": "deepseek",
            "output_cost_per_token": 4e-07
        },
        "gemini-2.5-flash": {
            "input_cost_per_token": 3e-07,
            "litellm_provider": "vertex_ai-language-models",
            "output_cost_per_token": 2.5e-06
        },
        "vertex_ai/claude-opus-4-5": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "vertex_ai-anthropic_models",
            "output_cost_per_token": 9e-05
        },
        "azure/gpt-5.5": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "azure",
            "output_cost_per_token": 9e-05
        },
        "us.anthropic.claude-sonnet-4-5": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "bedrock_converse",
            "output_cost_per_token": 9e-05
        },
        "openrouter/anthropic/claude-opus-4-5": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "openrouter",
            "output_cost_per_token": 9e-05
        },
        "chatgpt/gpt-5.5": {
            "litellm_provider": "chatgpt",
            "mode": "responses"
        },
        "low/1024-x-1024/gpt-image-9": {
            "input_cost_per_token": 9e-06,
            "litellm_provider": "openai",
            "output_cost_per_token": 9e-05
        },
        "gpt-image-only": {
            "input_cost_per_image": 0.04,
            "litellm_provider": "openai"
        },
        "o9-broken": {
            "input_cost_per_token": "cheap",
            "litellm_provider": "openai",
            "output_cost_per_token": 1e-06
        },
        "o9-negative": {
            "input_cost_per_token": -1e-06,
            "litellm_provider": "openai",
            "output_cost_per_token": 1e-06
        }
    }"#;

    fn table() -> PriceTable {
        read_litellm(TABLE.as_bytes()).expect("the made-up table reads")
    }

    /// Input, cache read, cache write and output, in micro-dollars per
    /// million tokens.
    fn in_micro_dollars(prices: &TokenPrices) -> [u64; 4] {
        [
            prices.input.micro_dollars(),
            prices.cached.micro_dollars(),
            prices.cache_written.micro_dollars(),
            prices.output.micro_dollars(),
        ]
    }

    fn standard(table: &PriceTable, model: &str) -> Option<[u64; 4]> {
        table
            .price_of(model)
            .map(|price| in_micro_dollars(price.standard()))
    }

    fn long_prompt(table: &PriceTable, model: &str) -> Option<(u64, [u64; 4])> {
        let long = table.price_of(model)?.long_prompt()?;

        Some((long.over_tokens(), in_micro_dollars(long.prices())))
    }

    #[test]
    fn a_plain_key_of_the_models_maker_is_read() {
        let table = table();

        assert_eq!(
            standard(&table, "claude-sonnet-4-5"),
            Some([3_000_000, 300_000, 3_750_000, 15_000_000])
        );
        assert_eq!(
            standard(&table, "claude-sonnet-4-5-20250929"),
            standard(&table, "claude-sonnet-4-5"),
            "a dated snapshot finds its model"
        );
        assert_eq!(long_prompt(&table, "claude-sonnet-4-5"), None);
    }

    #[test]
    fn a_key_with_its_makers_provider_is_read_and_a_plain_key_wins() {
        let table = table();

        assert_eq!(
            standard(&table, "deepseek-v3.2"),
            Some([280_000, 28_000, 280_000, 400_000])
        );
        assert_eq!(
            standard(&table, "deepseek-chat"),
            Some([280_000, 28_000, 280_000, 420_000]),
            "the plain key, not deepseek/deepseek-chat"
        );
        assert_eq!(table.price_of("deepseek/deepseek-v3.2"), None);
    }

    #[test]
    fn the_keys_of_clouds_and_resellers_are_left_out() {
        let table = table();

        for model in [
            "gemini-2.5-flash",
            "claude-opus-4-5",
            "anthropic/claude-opus-4-5",
            "us.anthropic.claude-sonnet-4-5",
            "1024-x-1024/gpt-image-9",
            "gpt-image-9",
        ] {
            assert_eq!(table.price_of(model), None, "{model}");
        }
        assert_eq!(
            standard(&table, "gpt-5.5"),
            Some([5_000_000, 500_000, 5_000_000, 30_000_000]),
            "Azure's price of the same model does not mix in"
        );
    }

    #[test]
    fn the_long_prompt_prices_are_read_with_their_limit() {
        let table = table();

        assert_eq!(
            standard(&table, "claude-haiku-5-5"),
            Some([100_000, 10_000, 125_000, 500_000])
        );
        assert_eq!(
            long_prompt(&table, "claude-haiku-5-5"),
            Some((100_000, [500_000, 50_000, 625_000, 2_500_000]))
        );
        // No long cache write price: the long input price.
        assert_eq!(
            long_prompt(&table, "gpt-5.5"),
            Some((272_000, [10_000_000, 1_000_000, 10_000_000, 45_000_000]))
        );
    }

    #[test]
    fn of_two_long_prompt_limits_the_lowest_is_used() {
        let table = table();

        assert_eq!(
            long_prompt(&table, "gemini-pro-long"),
            Some((128_000, [2_000_000, 2_000_000, 2_000_000, 4_000_000]))
        );
    }

    /// DeepSeek charges a cache write as plain input; the file writes 0.
    #[test]
    fn a_cache_write_never_costs_less_than_input() {
        let table = table();

        let written = table
            .price_of("deepseek-v3.2")
            .map(|price| price.standard().cache_written.micro_dollars());

        assert_eq!(written, Some(280_000));
    }

    #[test]
    fn a_model_with_no_cache_prices_pays_its_input_price_for_the_cache() {
        let table = table();

        assert_eq!(
            standard(&table, "gpt-5-pro"),
            Some([15_000_000, 15_000_000, 15_000_000, 120_000_000])
        );
    }

    #[test]
    fn an_entry_with_no_input_or_output_price_is_left_out() {
        let table = table();

        for model in [
            "sample_spec",
            "gpt-5.5-chatgpt",
            "gpt-image-only",
            "o9-broken",
            "o9-negative",
        ] {
            assert_eq!(table.price_of(model), None, "{model}");
        }
        assert_eq!(table.len(), 7);
    }

    #[test]
    fn a_broken_file_is_not_what_the_code_expects() {
        for broken in [
            "<html>rate limited</html>",
            "[1, 2]",
            r#"{"model": {"litellm_provider": "openai"}}"#,
            "",
        ] {
            assert!(
                matches!(
                    read_litellm(broken.as_bytes()),
                    Err(SourceError::UnexpectedAnswer(_))
                ),
                "{broken}"
            );
        }
    }

    #[test]
    fn a_price_per_token_is_rounded_to_whole_micro_dollars_per_million() {
        // Real prices of the LiteLLM file, whose floats are not exact.
        for (per_token, per_million) in [
            (3e-06, 3_000_000),
            (1.875e-06, 1_875_000),
            (1.5625e-05, 15_625_000),
            (2.8e-08, 28_000),
            (4.4e-08, 44_000),
            (1.32e-06, 1_320_000),
            (5e-09, 5_000),
            (6e-09, 6_000),
            (0.00012, 120_000_000),
            (0.0, 0),
        ] {
            assert_eq!(
                micro_dollars_per_million(per_token),
                Some(per_million),
                "{per_token}"
            );
        }
        assert_eq!(micro_dollars_per_million(4e-13), Some(0));
        assert_eq!(micro_dollars_per_million(6e-13), Some(1));
        assert_eq!(micro_dollars_per_million(-1e-06), None);
        assert_eq!(micro_dollars_per_million(1e30), None);
    }

    #[test]
    fn the_model_of_a_price_is_its_name_without_the_provider() {
        let table = table();

        let model = table.price_of("deepseek-v3.2").map(ModelPrice::model);

        assert_eq!(model, Some("deepseek-v3.2"));
    }
}
