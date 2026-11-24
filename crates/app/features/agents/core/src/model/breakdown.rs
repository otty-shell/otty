//! What a number of tokens or a sum of money of the usage block is made
//! of, as its tooltip lists it (issue 16 of spec
//! 2026-11-23-accounts-apart-from-profiles): one kind per line, its name
//! and its number.

use otty_agents::{Dollars, FetchedPrices, MoneyTotal, TokenCount};
use otty_app_i18n::{self as i18n, Key};

use super::numbers::{dollars_text, short_count};

/// What some tokens and their money are made of: the lines of the
/// tooltips of a total, a bar or a cell of the usage block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Breakdown {
    /// The kinds of the tokens: in, cache write, cached and out, a kind
    /// with none left out.
    pub(crate) tokens: Vec<BreakdownLine>,
    /// The money of each kind at API prices, such as "~$1.20", then the
    /// money the agent wrote, which has no parts. A kind with no money is
    /// left out, and there is none before the prices are read.
    pub(crate) money: Vec<BreakdownLine>,
}

impl Breakdown {
    /// What `tokens` and their `money` are made of; no money before the
    /// `prices` are read.
    pub(crate) fn of(
        tokens: &TokenCount,
        money: &MoneyTotal,
        prices: Option<&FetchedPrices>,
    ) -> Self {
        let money = match prices {
            Some(_) => money_lines(money),
            None => Vec::new(),
        };

        Self {
            tokens: token_lines(tokens),
            money,
        }
    }
}

/// One line of such a tooltip: the name of a kind at the left and its
/// number at the right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BreakdownLine {
    /// The kind, such as "cached".
    pub(crate) name: &'static str,
    /// Its tokens, such as "1.2M", or its money, such as "~$0.40".
    pub(crate) value: String,
}

impl BreakdownLine {
    /// The line of the kind `key` and its `value`.
    fn of(key: Key, value: String) -> Self {
        Self {
            name: i18n::t(key),
            value,
        }
    }
}

/// The kinds of `tokens` that are not zero. The input is the part not
/// written to the cache, so the kinds add up to the tokens shown.
fn token_lines(tokens: &TokenCount) -> Vec<BreakdownLine> {
    let written = tokens.cache_written().min(tokens.input());
    let kinds = [
        (Key::AgentsKindInput, tokens.input() - written),
        (Key::AgentsKindCacheWritten, written),
        (Key::AgentsKindCached, tokens.cached()),
        (Key::AgentsKindOutput, tokens.output()),
    ];

    kinds
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(key, count)| BreakdownLine::of(key, short_count(count)))
        .collect()
}

/// The kinds of the money of `money` at API prices that are not zero,
/// each an estimate with "~", then the money the agent wrote.
fn money_lines(money: &MoneyTotal) -> Vec<BreakdownLine> {
    let priced = money.priced_by_kind();
    let kinds = [
        (Key::AgentsKindInput, priced.input()),
        (Key::AgentsKindCacheWritten, priced.cache_written()),
        (Key::AgentsKindCached, priced.cached()),
        (Key::AgentsKindOutput, priced.output()),
    ];

    let mut lines: Vec<BreakdownLine> = kinds
        .into_iter()
        .filter(|(_, amount)| has_money(*amount))
        .map(|(key, amount)| {
            BreakdownLine::of(key, format!("~{}", dollars_text(amount)))
        })
        .collect();
    if has_money(money.written_cost()) {
        lines.push(BreakdownLine::of(
            Key::AgentsMoneyWrittenByAgent,
            dollars_text(money.written_cost()),
        ));
    }

    lines
}

/// Whether `amount` is any money at all.
fn has_money(amount: Dollars) -> bool {
    amount.micro_dollars() > 0
}

#[cfg(test)]
mod tests {
    use otty_agents::{Dollars, MoneyByKind};

    use super::*;
    use crate::testing::{at, fetched_prices};

    fn line(key: Key, value: &str) -> BreakdownLine {
        BreakdownLine {
            name: i18n::t(key),
            value: String::from(value),
        }
    }

    fn dollars(cents: u64) -> Dollars {
        Dollars::from_micro_dollars(cents * 10_000)
    }

    /// Money at API prices of each kind, in cents.
    fn priced(
        input: u64,
        cache_written: u64,
        cached: u64,
        output: u64,
    ) -> MoneyTotal {
        let by_kind = MoneyByKind::new(
            dollars(input),
            dollars(cache_written),
            dollars(cached),
            dollars(output),
        );

        MoneyTotal::at_api_prices(by_kind, 1)
    }

    fn of(tokens: TokenCount, money: MoneyTotal) -> Breakdown {
        Breakdown::of(&tokens, &money, Some(&fetched_prices(at(10))))
    }

    /// The input lists the part not written to the cache, so the kinds
    /// add up to the tokens shown: in, cache write, cached, out.
    #[test]
    fn the_tokens_list_each_kind_one_per_line() {
        let tokens =
            TokenCount::new(1_200, 1_000_000, 4_500).with_cache_written(200);

        let breakdown = of(tokens, MoneyTotal::default());

        assert_eq!(
            breakdown.tokens,
            [
                line(Key::AgentsKindInput, "1K"),
                line(Key::AgentsKindCacheWritten, "200"),
                line(Key::AgentsKindCached, "1M"),
                line(Key::AgentsKindOutput, "4.5K"),
            ]
        );
    }

    #[test]
    fn a_kind_with_no_tokens_is_left_out() {
        let breakdown = of(TokenCount::new(10, 0, 5), MoneyTotal::default());

        assert_eq!(
            breakdown.tokens,
            [
                line(Key::AgentsKindInput, "10"),
                line(Key::AgentsKindOutput, "5"),
            ]
        );
    }

    /// The money at API prices is an estimate, each kind with its "~"; a
    /// kind with no money is left out.
    #[test]
    fn the_money_lists_each_kind_at_api_prices() {
        let breakdown = of(TokenCount::new(10, 20, 5), priced(120, 0, 40, 200));

        assert_eq!(
            breakdown.money,
            [
                line(Key::AgentsKindInput, "~$1.20"),
                line(Key::AgentsKindCached, "~$0.40"),
                line(Key::AgentsKindOutput, "~$2.00"),
            ]
        );
    }

    /// The cost the agent wrote has no parts: one line says who wrote it.
    #[test]
    fn money_the_agent_wrote_says_so_with_no_parts() {
        let written = MoneyTotal::written(dollars(50));

        let breakdown = of(TokenCount::new(10, 20, 5), written);

        assert_eq!(
            breakdown.money,
            [line(Key::AgentsMoneyWrittenByAgent, "$0.50")]
        );
    }

    /// A mix, such as a project of Codex and OpenCode, lists the kinds of
    /// the priced share and then the written share.
    #[test]
    fn a_mix_lists_the_priced_kinds_and_the_written_share() {
        let mix = priced(100, 30, 0, 0) + MoneyTotal::written(dollars(50));

        let breakdown = of(TokenCount::new(10, 20, 5), mix);

        assert_eq!(
            breakdown.money,
            [
                line(Key::AgentsKindInput, "~$1.00"),
                line(Key::AgentsKindCacheWritten, "~$0.30"),
                line(Key::AgentsMoneyWrittenByAgent, "$0.50"),
            ]
        );
    }

    /// Before the prices are read, and for money with no price at all,
    /// there is no money to list.
    #[test]
    fn money_not_read_yet_or_with_no_price_lists_nothing() {
        let tokens = TokenCount::new(10, 0, 0);

        let before = Breakdown::of(&tokens, &priced(100, 0, 0, 0), None);
        let unpriced = of(tokens, MoneyTotal::unpriced(2));

        assert!(before.money.is_empty());
        assert_eq!(before.tokens, [line(Key::AgentsKindInput, "10")]);
        assert!(unpriced.money.is_empty());
    }

    #[test]
    fn nothing_spent_lists_nothing() {
        let breakdown = of(TokenCount::default(), MoneyTotal::default());

        assert_eq!(breakdown, Breakdown::default());
    }
}
