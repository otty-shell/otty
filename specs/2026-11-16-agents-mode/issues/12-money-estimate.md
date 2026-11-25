# 12 Money for API key profiles: the price table and the estimate

Status: ready-for-agent
Blocked by: 09, 11

## Why

A profile billed by an API key has no limits; what matters is the money it
spent (spec Decision 9). Without an admin key OTTY can only estimate it from
the tokens of the session files and the list prices.

## What to do

1. A price table in `otty-agents`, as data in one file: for each model id
   (or model family prefix, when the provider prices a family the same), the
   price per million input, cached input (read and write where they differ)
   and output tokens, in US dollars, and the date the prices were taken.
   Fill it from the providers' public price pages for the Claude and OpenAI
   models the agents use; put each page's address in a comment. Read
   CodexBar's `CostUsagePricing` for the model ids and the rules it uses
   (for example the Codex "fast" tier) and follow it where it fits.
2. A function that turns the per-model tokens of `Spend` (issue 09) into
   money: today, last 7 days, this month, per day for the last 30 days, per
   model this month. A model with no price keeps its tokens and is counted
   as "no price"; it adds nothing to the money and the dashboard says how
   many requests had no price. Nothing is guessed from a similar model.
3. The money result says it is an estimate and from which price table date.
4. It runs only for API key profiles (issue 11).

## Done when

- Tests, written first, cover: a known model; a family prefix; cached input
  priced apart; a model with no price; the day and month edges in the user's
  time zone (the time is passed in, as in issue 09).
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built in `otty-agents`, tests first and seen failing. Money is in US
dollars as whole micro-dollars (`Dollars`, a `u64`), so adding many small
prices never drifts; a cost is worked out in `u128` and rounded to the
nearest micro-dollar once per model and day. `PriceTable` holds rows of
`ModelPrice` (input, cached input, input written to the cache, output, per
million tokens) and the date they were taken. A row prices its model id
and the dated snapshots of it (`claude-opus-4-5-20251101`,
`gpt-5-2025-08-07`), and nothing else: `gpt-5` does not price `gpt-5-mini`.
For Claude Code the input written to the cache is priced as a cache write
and the rest of the input as input. A row may also have long prompt prices
(`with_long_prompt`): Claude Haiku 5.5 for a prompt over 100,000 tokens,
and the OpenAI models with a long context price for a prompt over 272,000
tokens. The prompt is the input of one request with its cached part (sent,
written to and read from the cache), as both pages count it. The tally
sorts each request when it counts it: `ModelTokens::long_prompts` is the
part of a model's tokens spent in long prompts (the limit comes from the
bundled rows), and `ModelPrice::cost` prices that part at the long prompt
prices and the rest at the standard ones. The bundled table is plain Rust data in
`src/bundled_prices.rs` (`bundled_prices()`), taken on 2026-11-16 from
https://platform.claude.com/docs/en/about-claude/pricing (ids from the
models overview and the model ids page) and
https://developers.openai.com/api/docs/pricing (and the `gpt-5.6` model
page for its alias), checked against CodexBar's `CostUsagePricing`.

`MoneyEstimate::new(&spend, &prices)` gives the money today, in 7 days and
this month, each of the 30 days, and each model this month (the most
first, models with no price last), with the date of the prices. A model
with no price, or with no name, keeps its tokens, has no cost and adds its
requests to "requests with no price"; a model with no tokens costs $0. For
that count `ModelTokens` now also keeps the number of requests (answers)
of the model, counted in `spend_tally`. The app is unchanged apart from
two test helpers: it does not keep the detected billing yet, so working
out the money only for API key profiles is left to issue 16, which draws
it.

Deviations: fast mode, the Batch API and US-only inference are not
applied, because the session files do not say which requests used them,
and every Claude cache write is priced as a 5-minute write (the files give
one number for 5-minute and 1-hour writes). Codex writes no cache-write
count, so OpenAI cache writes are priced as input, as CodexBar does. The
OpenAI page does not say in so many words that cached input counts toward
the 272K limit; it is counted, as OpenAI's input tokens include the cached
ones and as CodexBar reads the rule. Models retired from the Claude API and the shut-down
Codex models (`gpt-5-codex` to `gpt-5.2-codex`) are left out. The page
says GPT-5.6 Sol's price is promotional until 2026-11-30; the table will
need an update after that.
