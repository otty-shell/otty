//! The list prices bundled with OTTY, for the Claude and OpenAI models the
//! agents use (spec Decision 9).

use crate::local_time::CalendarDate;
use crate::price_table::{LongPromptPrice, ModelPrice, PriceTable};

/// The day the prices below were read from the providers' price pages.
const TAKEN_ON: CalendarDate = CalendarDate::new(2026, 11, 16);

/// The prices in micro-dollars per million tokens: `1_250_000` is $1.25.
///
/// The columns are: input, cached input (a cache read), input written to
/// the cache, output. A row prices its model id and the dated snapshots of
/// it (see [`PriceTable::price_of`]); a row with `.with_long_prompt` has
/// other prices for a request whose prompt is over the limit. The fast
/// mode, the Batch API, US-only inference and regional endpoints are not
/// here, because the session files do not say which requests used them.
#[rustfmt::skip]
const PRICES: &[ModelPrice] = &[
    // Anthropic, read on 2026-11-16 from
    // https://platform.claude.com/docs/en/about-claude/pricing
    // and the model ids from
    // https://platform.claude.com/docs/en/about-claude/models/overview and
    // https://platform.claude.com/docs/en/about-claude/models/model-ids-and-versions
    //
    // A cache write is priced as a 5-minute write: the session files give
    // one number for all the writes. Claude Haiku 5.5 costs more for a
    // request whose prompt is over 100,000 tokens; the page counts "all of
    // its input tokens, including cache reads and cache writes". Models
    // retired from the Claude API (Opus 4.1, Opus 4, Sonnet 4) are left
    // out.
    //
    //              model                      input      cached  cache write       output
    ModelPrice::new("claude-fable-5-2",    8_000_000,    200_000,  10_000_000,   40_000_000),
    ModelPrice::new("claude-mythos-5-2",   8_000_000,    200_000,  10_000_000,   40_000_000),
    ModelPrice::new("claude-fable-5-1",   10_000_000,    250_000,  12_500_000,   50_000_000),
    ModelPrice::new("claude-mythos-5-1",  10_000_000,    250_000,  12_500_000,   50_000_000),
    ModelPrice::new("claude-fable-5",     10_000_000,  1_000_000,  12_500_000,   50_000_000),
    ModelPrice::new("claude-mythos-5",    10_000_000,  1_000_000,  12_500_000,   50_000_000),
    ModelPrice::new("claude-opus-5-5",     4_000_000,    200_000,   5_000_000,   20_000_000),
    ModelPrice::new("claude-opus-5",       5_000_000,    500_000,   6_250_000,   25_000_000),
    ModelPrice::new("claude-opus-4-8",     5_000_000,    500_000,   6_250_000,   25_000_000),
    ModelPrice::new("claude-opus-4-7",     5_000_000,    500_000,   6_250_000,   25_000_000),
    ModelPrice::new("claude-opus-4-6",     5_000_000,    500_000,   6_250_000,   25_000_000),
    ModelPrice::new("claude-opus-4-5",     5_000_000,    500_000,   6_250_000,   25_000_000),
    ModelPrice::new("claude-sonnet-5-5",   2_000_000,    100_000,   2_500_000,   10_000_000),
    ModelPrice::new("claude-sonnet-5",     2_000_000,    200_000,   2_500_000,   10_000_000),
    ModelPrice::new("claude-sonnet-4-6",   3_000_000,    300_000,   3_750_000,   15_000_000),
    ModelPrice::new("claude-sonnet-4-5",   3_000_000,    300_000,   3_750_000,   15_000_000),
    ModelPrice::new("claude-haiku-5-5",      100_000,     10_000,     125_000,      500_000)
        .with_long_prompt(100_000,           500_000,     50_000,     625_000,    2_500_000),
    ModelPrice::new("claude-haiku-4-5",    1_000_000,    100_000,   1_250_000,    5_000_000),
    //
    // OpenAI, read on 2026-11-16 from the Standard prices (short context)
    // of https://developers.openai.com/api/docs/pricing; `gpt-5.3-codex`
    // and the Cyber models are in its Specialized models table.
    //
    // A model with a long context price costs more for a request with
    // more than 272K (272,000) input tokens: "Short context: ≤272K input
    // tokens. Long context: >272K input tokens." The cached input is
    // counted in, as OpenAI's input tokens include the cached ones (and
    // as CodexBar reads the rule).
    //
    // Codex does not write down the input it wrote to the cache, so the
    // cache write column is never used for these models; it holds the
    // 30-minute write price where the page gives one, else the input
    // price. A model with no cached input price (the Pro models) gives no
    // discount, so its cached column holds the input price. The page says
    // GPT-5.6 Sol's price is promotional until 2026-11-30. `gpt-5.6` is
    // the alias of `gpt-5.6-sol`, as
    // https://developers.openai.com/api/docs/models/gpt-5.6 says.
    // The Codex models that OpenAI has shut down (such as `gpt-5-codex`
    // and `gpt-5.2-codex`) are left out: they can not be in the last 30
    // days.
    //
    //              model                      input      cached  cache write       output
    ModelPrice::new("gpt-6-astra",        10_000_000,  1_000_000,  12_500_000,   50_000_000)
        .with_long_prompt(272_000,        20_000_000,  2_000_000,  25_000_000,   75_000_000),
    ModelPrice::new("gpt-6.1-sol",         2_000_000,    100_000,   2_500_000,   10_000_000)
        .with_long_prompt(272_000,         4_000_000,    200_000,   5_000_000,   15_000_000),
    ModelPrice::new("gpt-6.1-terra",       1_000_000,     50_000,   1_250_000,    5_000_000)
        .with_long_prompt(272_000,         2_000_000,    100_000,   2_500_000,    7_500_000),
    ModelPrice::new("gpt-6-sol",           2_000_000,    200_000,   2_500_000,   10_000_000)
        .with_long_prompt(272_000,         4_000_000,    400_000,   5_000_000,   15_000_000),
    ModelPrice::new("gpt-6-luna",            100_000,     10_000,     125_000,      500_000)
        .with_long_prompt(272_000,           200_000,     20_000,     250_000,      750_000),
    ModelPrice::new("gpt-6-cyber",        20_000_000,  2_000_000,  25_000_000,  100_000_000)
        .with_long_prompt(272_000,        40_000_000,  4_000_000,  40_000_000,  150_000_000),
    ModelPrice::new("gpt-5.6-sol",         4_000_000,    400_000,   5_000_000,   20_000_000)
        .with_long_prompt(272_000,         8_000_000,    800_000,  10_000_000,   30_000_000),
    ModelPrice::new("gpt-5.6",             4_000_000,    400_000,   5_000_000,   20_000_000)
        .with_long_prompt(272_000,         8_000_000,    800_000,  10_000_000,   30_000_000),
    ModelPrice::new("gpt-5.6-terra",       2_000_000,    200_000,   2_500_000,   12_000_000)
        .with_long_prompt(272_000,         4_000_000,    400_000,   5_000_000,   18_000_000),
    ModelPrice::new("gpt-5.6-luna",          200_000,     20_000,     250_000,    1_200_000)
        .with_long_prompt(272_000,           400_000,     40_000,     500_000,    1_800_000),
    ModelPrice::new("gpt-5.6-cyber",      12_500_000,  1_250_000,  15_625_000,   75_000_000),
    ModelPrice::new("gpt-5.5",             5_000_000,    500_000,   5_000_000,   30_000_000)
        .with_long_prompt(272_000,        10_000_000,  1_000_000,  10_000_000,   45_000_000),
    ModelPrice::new("gpt-5.5-pro",        30_000_000, 30_000_000,  30_000_000,  180_000_000)
        .with_long_prompt(272_000,        60_000_000, 60_000_000,  60_000_000,  270_000_000),
    ModelPrice::new("gpt-5.4",             2_500_000,    250_000,   2_500_000,   15_000_000)
        .with_long_prompt(272_000,         5_000_000,    500_000,   5_000_000,   22_500_000),
    ModelPrice::new("gpt-5.4-pro",        30_000_000, 30_000_000,  30_000_000,  180_000_000)
        .with_long_prompt(272_000,        60_000_000, 60_000_000,  60_000_000,  270_000_000),
    ModelPrice::new("gpt-5.4-mini",          750_000,     75_000,     750_000,    4_500_000),
    ModelPrice::new("gpt-5.4-nano",          200_000,     20_000,     200_000,    1_250_000),
    ModelPrice::new("gpt-5.3-codex",       1_750_000,    175_000,   1_750_000,   14_000_000),
    ModelPrice::new("gpt-5.2",             1_750_000,    175_000,   1_750_000,   14_000_000),
    ModelPrice::new("gpt-5.2-pro",        21_000_000, 21_000_000,  21_000_000,  168_000_000),
    ModelPrice::new("gpt-5.1",             1_250_000,    125_000,   1_250_000,   10_000_000),
    ModelPrice::new("gpt-5",               1_250_000,    125_000,   1_250_000,   10_000_000),
    ModelPrice::new("gpt-5-mini",            250_000,     25_000,     250_000,    2_000_000),
    ModelPrice::new("gpt-5-nano",             50_000,      5_000,      50_000,      400_000),
    ModelPrice::new("gpt-5-pro",          15_000_000, 15_000_000,  15_000_000,  120_000_000),
];

/// The prompt limit above which `model` costs more, by the bundled prices;
/// `None` when it has one price for every prompt or no price at all.
pub(crate) fn long_prompt_limit(model: &str) -> Option<u64> {
    PRICES
        .iter()
        .find(|price| price.covers(model))
        .and_then(ModelPrice::long_prompt)
        .map(LongPromptPrice::over_tokens)
}

/// The list prices bundled with OTTY and the date they were taken.
///
/// ```
/// let prices = otty_agents::bundled_prices();
///
/// assert!(prices.price_of("claude-opus-5-5").is_some());
/// ```
pub fn bundled_prices() -> PriceTable {
    PriceTable::new(TAKEN_ON, PRICES.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::price_table::TokenPrices;

    fn in_micro_dollars(prices: &TokenPrices) -> [u64; 4] {
        [
            prices.input().micro_dollars(),
            prices.cached().micro_dollars(),
            prices.cache_written().micro_dollars(),
            prices.output().micro_dollars(),
        ]
    }

    fn dollars_per_million(model: &str) -> Option<[u64; 4]> {
        bundled_prices()
            .price_of(model)
            .map(|price| in_micro_dollars(price.standard()))
    }

    /// The prompt limit and the prices above it.
    fn long_prompt(model: &str) -> Option<(u64, [u64; 4])> {
        let prices = bundled_prices();
        let long = prices.price_of(model)?.long_prompt()?;

        Some((long.over_tokens(), in_micro_dollars(long.prices())))
    }

    #[test]
    fn each_row_is_found_by_its_own_id_only() {
        let prices = bundled_prices();

        for row in PRICES {
            let found = prices.price_of(row.model()).map(ModelPrice::model);

            assert_eq!(found, Some(row.model()));
        }
        assert!(!PRICES.is_empty());
    }

    #[test]
    fn cache_reads_cost_no_more_than_input_and_writes_no_less() {
        for row in PRICES {
            let long = row.long_prompt().map(LongPromptPrice::prices);
            for prices in [Some(row.standard()), long].into_iter().flatten() {
                assert!(prices.cached() <= prices.input(), "{}", row.model());
                assert!(
                    prices.cache_written() >= prices.input(),
                    "{}",
                    row.model()
                );
            }
        }
    }

    #[test]
    fn the_prompt_limit_comes_from_the_bundled_rows() {
        assert_eq!(long_prompt_limit("claude-haiku-5-5"), Some(100_000));
        assert_eq!(long_prompt_limit("gpt-5.6"), Some(272_000));
        assert_eq!(long_prompt_limit("gpt-6.1-sol-2026-10-01"), Some(272_000));
        assert_eq!(long_prompt_limit("claude-opus-5-5"), None);
        assert_eq!(long_prompt_limit("gpt-5.4-mini"), None);
        assert_eq!(long_prompt_limit("unknown"), None);
    }

    #[test]
    fn current_claude_models_are_priced() {
        assert_eq!(
            dollars_per_million("claude-opus-5-5"),
            Some([4_000_000, 200_000, 5_000_000, 20_000_000])
        );
        assert_eq!(
            dollars_per_million("claude-sonnet-4-5-20250929"),
            Some([3_000_000, 300_000, 3_750_000, 15_000_000])
        );
    }

    #[test]
    fn current_openai_models_are_priced() {
        assert_eq!(
            dollars_per_million("gpt-6.1-sol"),
            Some([2_000_000, 100_000, 2_500_000, 10_000_000])
        );
        assert_eq!(
            dollars_per_million("gpt-5.6"),
            dollars_per_million("gpt-5.6-sol")
        );
    }

    #[test]
    fn models_priced_by_the_length_of_the_prompt_have_both_prices() {
        assert_eq!(
            dollars_per_million("claude-haiku-5-5"),
            Some([100_000, 10_000, 125_000, 500_000])
        );
        assert_eq!(
            long_prompt("claude-haiku-5-5"),
            Some((100_000, [500_000, 50_000, 625_000, 2_500_000]))
        );
        assert_eq!(
            long_prompt("gpt-6.1-sol"),
            Some((272_000, [4_000_000, 200_000, 5_000_000, 15_000_000]))
        );
    }

    #[test]
    fn the_table_says_when_its_prices_were_taken() {
        assert_eq!(bundled_prices().taken_on(), TAKEN_ON);
    }
}
