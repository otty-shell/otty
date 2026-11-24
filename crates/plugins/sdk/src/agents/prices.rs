//! The money of an answer as an agent part gives it (spec
//! 2026-11-23-accounts-apart-from-profiles, Decision 3): the cost the
//! agent wrote, as it is, else the tokens at the API price of the model
//! that OTTY's price list gives, marked "at API prices", kind by kind
//! next to their total (issue 16).

use crate::agents::{
    AnswerCost, ModelPrice, PricedCost, ProviderRef, TokenCount,
};

/// How many tokens a price is for.
const TOKENS_PER_PRICE: u128 = 1_000_000;

/// The API price of `model`, as the provider names it, served by
/// `provider`, from OTTY's price list: that provider's price, else the
/// maker's. `None` when the list has no price for it, or OTTY has no list
/// yet.
pub fn price_of(
    model: &str,
    provider: Option<&ProviderRef>,
) -> Option<ModelPrice> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::bindings::agents_agent::otty::agents_agent::prices::price_of(
            model, provider,
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::test_host::price_of(model, provider)
    }
}

/// What one answer cost: `written`, the cost the agent wrote in
/// millionths of a US dollar, as it is, with no parts; else its `tokens`
/// at the API price of `model` served by `provider` ([`price_of`]),
/// marked at API prices, each kind apart ([`cost_at`]). `None` when the
/// agent wrote no cost and the model has no price.
///
/// ```no_run
/// use otty_plugin_sdk::agents::{AnswerCost, TokenCount, answer_cost};
///
/// let tokens = TokenCount {
///     input: 10,
///     cached: 0,
///     cache_written: 0,
///     output: 5,
///     reasoning: 0,
/// };
///
/// let cost = answer_cost(Some(1_200), Some("gpt-5"), None, &tokens);
///
/// assert_eq!(cost, Some(AnswerCost::Written(1_200)));
/// ```
pub fn answer_cost(
    written: Option<u64>,
    model: Option<&str>,
    provider: Option<&ProviderRef>,
    tokens: &TokenCount,
) -> Option<AnswerCost> {
    if let Some(written) = written {
        return Some(AnswerCost::Written(written));
    }

    let price = price_of(model?, provider)?;

    Some(AnswerCost::AtApiPrices(cost_at(&price, tokens)))
}

/// What the `tokens` of one request cost at `price`, in millionths of a
/// US dollar, kind by kind and in all: at the long prompt prices when its
/// prompt, the input with the part read from the cache, is over their
/// limit, else at the standard ones. The input written to the cache is
/// priced as a cache write, the rest of the input as input. Each kind is
/// rounded to the nearest millionth on its own, and the total adds the
/// kinds up, so they always tell the same sum.
///
/// ```
/// use otty_plugin_sdk::agents::{
///     ModelPrice, TokenCount, TokenPrices, cost_at,
/// };
///
/// let price = ModelPrice {
///     standard: TokenPrices {
///         input: 2_000_000,
///         cached: 0,
///         cache_written: 0,
///         output: 0,
///     },
///     long_prompt: None,
/// };
/// let tokens = TokenCount {
///     input: 500_000,
///     cached: 0,
///     cache_written: 0,
///     output: 0,
///     reasoning: 0,
/// };
///
/// let cost = cost_at(&price, &tokens);
///
/// assert_eq!(cost.input, 1_000_000);
/// assert_eq!(cost.total, 1_000_000);
/// ```
pub fn cost_at(price: &ModelPrice, tokens: &TokenCount) -> PricedCost {
    let prompt = tokens.input.saturating_add(tokens.cached);
    let prices = match &price.long_prompt {
        Some(long) if prompt > long.over_tokens => &long.prices,
        _ => &price.standard,
    };

    let written = tokens.cache_written.min(tokens.input);
    let not_written = tokens.input - written;
    let input = price_of_count(not_written, prices.input);
    let cache_written = price_of_count(written, prices.cache_written);
    let cached = price_of_count(tokens.cached, prices.cached);
    let output = price_of_count(tokens.output, prices.output);

    PricedCost {
        total: input
            .saturating_add(cache_written)
            .saturating_add(cached)
            .saturating_add(output),
        input,
        cache_written,
        cached,
        output,
    }
}

/// `count` tokens at `price` per million, in millionths of a US dollar,
/// to the nearest. The product is wide enough that no count and price
/// overflow it.
fn price_of_count(count: u64, price: u64) -> u64 {
    let per_million_tokens = u128::from(count) * u128::from(price);
    let micro_dollars =
        (per_million_tokens + TOKENS_PER_PRICE / 2) / TOKENS_PER_PRICE;

    u64::try_from(micro_dollars).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{LongPromptPrice, PricedCost, TokenPrices};
    use crate::test_host::TestHost;
    use crate::testing::MadeUpPlugin;

    /// A dollar in millionths.
    const DOLLAR: u64 = 1_000_000;

    fn tokens(input: u64, cached: u64, output: u64) -> TokenCount {
        TokenCount {
            input,
            cached,
            cache_written: 0,
            output,
            reasoning: 0,
        }
    }

    fn per_million(
        input: u64,
        cached: u64,
        cache_written: u64,
        output: u64,
    ) -> TokenPrices {
        TokenPrices {
            input,
            cached,
            cache_written,
            output,
        }
    }

    /// The money of each kind, in millionths of a dollar, and their
    /// total.
    fn priced(
        input: u64,
        cache_written: u64,
        cached: u64,
        output: u64,
    ) -> PricedCost {
        PricedCost {
            total: input + cache_written + cached + output,
            input,
            cache_written,
            cached,
            output,
        }
    }

    /// $1 per million input tokens, $0.10 cached, $2 written to the
    /// cache, $10 output; four times that over a prompt of 100 tokens.
    fn price() -> ModelPrice {
        ModelPrice {
            standard: per_million(DOLLAR, DOLLAR / 10, 2 * DOLLAR, 10 * DOLLAR),
            long_prompt: Some(LongPromptPrice {
                over_tokens: 100,
                prices: per_million(4 * DOLLAR, 0, 0, 40 * DOLLAR),
            }),
        }
    }

    const AGENT: &str = "made-up-agent";

    /// The host of the made-up plugin, kept with its folder.
    fn host() -> (MadeUpPlugin, TestHost) {
        let plugin = MadeUpPlugin::new();
        let host = TestHost::new(plugin.folder());

        (plugin, host)
    }

    #[test]
    fn each_kind_of_token_meets_its_own_price() {
        let written = TokenCount {
            cache_written: 30,
            ..tokens(50, 40, 6)
        };

        let cost = cost_at(&price(), &written);

        // 20 input at $1, 30 written at $2, 40 cached at $0.10, 6 output
        // at $10: 20 + 60 + 4 + 60 millionths of a dollar.
        assert_eq!(cost, priced(20, 60, 4, 60));
        assert_eq!(cost.total, 144);
    }

    /// Issue 16 of spec 2026-11-23-accounts-apart-from-profiles: the money
    /// of each kind is rounded on its own, and the kinds add up to the
    /// total, so a tooltip of the parts never tells another sum.
    #[test]
    fn the_money_of_the_kinds_adds_up_to_the_total() {
        let tokens = TokenCount {
            input: 7,
            cached: 5,
            cache_written: 3,
            output: 1,
            reasoning: 0,
        };

        let cost = cost_at(&price(), &tokens);

        // 4 input at $1, 3 written at $2, 5 cached at $0.10 (half a
        // millionth, up to one), 1 output at $10.
        assert_eq!(cost, priced(4, 6, 1, 10));
        assert_eq!(
            cost.input + cost.cache_written + cost.cached + cost.output,
            cost.total
        );
    }

    #[test]
    fn a_prompt_over_the_limit_is_priced_as_a_long_prompt() {
        let at_the_limit = cost_at(&price(), &tokens(60, 40, 1));
        let over_it = cost_at(&price(), &tokens(61, 40, 1));

        assert_eq!(at_the_limit, priced(60, 0, 4, 10));
        assert_eq!(over_it, priced(4 * 61, 0, 0, 40));
    }

    #[test]
    fn the_cost_rounds_to_the_nearest_millionth() {
        let half_up = cost_at(&price(), &tokens(0, 5, 0));
        let half_down = cost_at(&price(), &tokens(0, 4, 0));

        assert_eq!((half_up.total, half_down.total), (1, 0));
    }

    /// Spec 2026-11-23-accounts-apart-from-profiles, Decision 3: the cost
    /// the agent wrote is given as it is, with no price asked, and with no
    /// parts (issue 16).
    #[test]
    fn the_cost_the_agent_wrote_is_given_as_it_is() {
        let (_plugin, host) = host();
        let host = host.with_price("m", None, price());

        let cost = host.as_agent(AGENT, || {
            answer_cost(Some(7), Some("m"), None, &tokens(1_000, 0, 0))
        });

        assert_eq!(cost, Some(AnswerCost::Written(7)));
    }

    #[test]
    fn with_no_written_cost_the_tokens_are_at_the_price_otty_gives() {
        let (_plugin, host) = host();
        let host = host.with_price("m", None, price());

        let cost = host.as_agent(AGENT, || {
            answer_cost(None, Some("m"), None, &tokens(1_000, 0, 0))
        });

        assert_eq!(cost, Some(AnswerCost::AtApiPrices(priced(4_000, 0, 0, 0))));
    }

    #[test]
    fn a_provider_s_own_price_comes_before_the_maker_s() {
        let openrouter = ProviderRef::Id("openrouter".to_string());
        let resold = ModelPrice {
            standard: per_million(2 * DOLLAR, 0, 0, 0),
            long_prompt: None,
        };
        let (_plugin, host) = host();
        let host = host.with_price("m", None, price()).with_price(
            "m",
            Some(openrouter.clone()),
            resold,
        );
        let deepseek = ProviderRef::Host("api.deepseek.com".to_string());

        let (resold, maker) = host.as_agent(AGENT, || {
            let resold = answer_cost(
                None,
                Some("m"),
                Some(&openrouter),
                &tokens(10, 0, 0),
            );
            let maker = answer_cost(
                None,
                Some("m"),
                Some(&deepseek),
                &tokens(10, 0, 0),
            );
            (resold, maker)
        });

        assert_eq!(resold, Some(AnswerCost::AtApiPrices(priced(20, 0, 0, 0))));
        assert_eq!(maker, Some(AnswerCost::AtApiPrices(priced(10, 0, 0, 0))));
    }

    #[test]
    fn a_model_with_no_price_or_no_name_has_no_cost() {
        let (_plugin, host) = host();
        let host = host.with_price("m", None, price());

        let costs = host.as_agent(AGENT, || {
            [
                answer_cost(None, Some("mystery"), None, &tokens(10, 0, 0)),
                answer_cost(None, None, None, &tokens(10, 0, 0)),
            ]
        });

        assert_eq!(costs, [None, None]);
    }
}
