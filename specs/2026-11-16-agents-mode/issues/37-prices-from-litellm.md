# 37 Prices from the LiteLLM table, no bundled table

Status: ready-for-agent
Blocked by: 36

## Why

The money estimate of an API key profile multiplies the tokens of the
session files by list prices (spec Decision 9). Those prices are bundled
in `crates/agents/src/bundled_prices.rs`, typed by hand from the
providers' price pages on 2026-11-16. A new model or a new price stays
wrong until the next OTTY release, and a model missing from the table has
"no price". Neither Anthropic nor OpenAI has an API that gives prices.

The owner chose (spec Decision 16, 2026-11-17) to take the prices from the
LiteLLM price table only, and to drop the bundled table. It is a JSON file in the open source LiteLLM project,
updated by its community almost every day and used by tools such as
`ccusage`. It also has the models of other providers, such as DeepSeek.

## What to do

1. **The source:**
   `https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`.
   - Read the file first and use the field names it really has. Expected:
     `input_cost_per_token`, `output_cost_per_token`,
     `cache_read_input_token_cost`, `cache_creation_input_token_cost`,
     `litellm_provider`, and the long-prompt fields such as
     `input_cost_per_token_above_200k_tokens`.
   - Map them onto `ModelPrice`: micro-dollars per million tokens, with the
     long-prompt prices where the file has them.
   - The file's prices are dollars per token as floats. Turn them into
     whole micro-dollars by rounding, and test the rounding on real
     values.
2. **Model names:** keys can be plain (`claude-sonnet-4-5`, `gpt-5`) or
   carry a provider (`anthropic/…`, `deepseek/…`, region prefixes).
   - A model id from the session files finds its price by the same rule
     `PriceTable::price_of` has today (the id and its dated snapshots).
   - Prefer the plain key, then the key of the agent's provider.
   - Leave out the keys of resellers and clouds (Bedrock, Vertex, Azure
     and the like) so their prices never mix in.
3. **When and how it is fetched:**
   - In the io crate, in the background, with the HTTP client the crate
     already has (no new dependency).
   - At most once a day, and only while Agents mode has a profile that
     uses the estimate.
   - The file and the time it was fetched are kept in OTTY's cache folder
     (`~/.cache/otty/prices/`). A start within a day uses the kept file.
   - A fetch that fails keeps the last good file.
   - Never send anything about the user: a plain GET, a generic user
     agent, no account data.
4. **Fallback order**, as a source walk like the others (spec Decision 5):
   the fetched file, then the kept file. There is no bundled table any
   more (owner, 2026-11-17).
   - A broken or unexpected file is a failure of that source, told once,
     as other real failures are (issue 24); the walk goes on.
   - When neither gives prices, the money rows of the Usage block show a
     dash "—" in place of each amount (not "$0.00", which would read as
     nothing spent; the budget bar is left out), and one muted line under them says the prices could not be
     read ("Could not get the prices. Press Refresh to try again."). The
     source row says why.
   - A model the file has no price for keeps today's "no price" row.
5. **Refresh** (the profile's button and Refresh all) fetches the prices
   again when the last fetch failed or there is no kept file, even within
   the day. A good fetch less than a day old is not fetched again.
6. **On screen:** "Where the numbers come from" names the prices:
   "LiteLLM prices, updated <date and time>", or the failure state. The
   tooltip says where the prices come from. The money stays an estimate in
   its wording.
7. **Remove the bundled table:** `bundled_prices.rs`, its date, its tests,
   and everything only it used. `PriceTable` and `ModelPrice` stay as the
   shape the fetched file is read into.

## Done when

- Tests, written first and seen failing, cover:
  - reading a small hand-made file in the LiteLLM layout: plain keys,
    provider keys, cloud keys left out, long-prompt prices, a model with
    no cache prices, a broken file;
  - the float-to-micro-dollar rounding;
  - the order (fetched, then kept) and no prices at all: dashes and the
    line under them;
  - Refresh fetching again after a failure, and not after a good fetch
    within the day;
  - fetching at most once a day;
  - the source name on screen.
- No test reaches the network.
- Run by hand: with network, the source says LiteLLM and a time. With the
  network off and the cache removed, the money rows show "—" with the line
  under it; turning the network on and pressing Refresh brings the money. A DeepSeek or
  new Claude model in the session files gets a price.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17 — built

What was built:

- `otty-agents`: `litellm.rs` reads the LiteLLM price table into a
  `PriceTable`; `price_source.rs` (`PriceSource`, `FetchedPrices`) walks the
  two price sources, LiteLLM then the kept copy, with the HTTP client the
  crate already has (a plain `GET`, the `otty/<version>` user agent, no
  headers). The copy is `~/.cache/otty/prices/litellm.json`; the time it
  was written is the time it was fetched. A new copy is written next to it
  and renamed into place, so a broken write never leaves half a file.
  `bundled_prices.rs` and its tests are gone; `PriceTable` has no date any
  more and `MoneyEstimate` no `prices_taken_on`. `SourceId` has
  `LiteLlmPrices` and `KeptPrices`. The source walk (`walk`) is now public,
  as the one walk of Decision 5, so the app's tests build price answers the
  same way.
- Long prompts: the tally knew the limit of each model from the bundled
  table. Now `ModelTokens` keeps the tokens over each of the limits the
  LiteLLM file uses (32k, 100k, 128k, 200k, 256k, 272k, 512k), and a price
  picks its own limit. A long prompt price with another limit is not used:
  that model is priced at its standard prices.
- io crate: `load_prices(cache_dir, fetch_again)` runs the walk on a
  thread; the binary answers `AgentsOutcome::LoadPrices`.
- Feature: `prices.rs` (`ListPrices`) keeps the latest walk and the last
  prices it gave. Prices are read only while a shown profile pays with an
  API key: at the start after the tokens (before the bills), with every read
  of the profiles on screen (Refresh too), and on opening the mode when the
  last read is older than five minutes. A good LiteLLM answer less than a day
  old is never read again; after a failed fetch the next read sets
  `fetch_again`. A price file that is broken or unexpected is told once
  ("Could not read the LiteLLM prices: …"); a network failure is not.
- Screen: with no prices the money rows show "—", the budget bar is left
  out, and a muted line says "Could not get the prices. Press Refresh to try
  again." While the first read runs, the dashes show without the line. The
  sources list the estimate (its tooltip: "… at the LiteLLM prices updated
  <date and time>, an estimate"), then "LiteLLM prices, updated <date and
  time>" (tooltip: where the prices come from, fetched at most once a day)
  and "Kept LiteLLM prices" as not needed, failed or answered. The
  statistics dialog shows "—" for the money of an API key profile with no
  prices. Six new strings in all ten locales; the estimate tooltip now takes
  `{time}`.

Choices:

- Keys: a key counts when its `litellm_provider` is a maker that sells its
  own models (anthropic, openai, deepseek, gemini, xai, mistral, moonshot,
  zai, minimax, dashscope), and the key is the plain model or
  `<that provider>/<model>`. The plain key wins. Every other key is left
  out: Bedrock (with its region prefixes), Vertex, Azure, OpenRouter and
  the other clouds and resellers, `chatgpt/…` (no prices), image keys such
  as `low/1024-x-1024/…`, and keys with no provider. On the 2026-11-17 file
  this gives 411 models, and the prices match the old bundled table.
- "The key of the agent's provider" is read as the key of the model's own
  provider: a DeepSeek model in a Claude Code session has no `anthropic/`
  key, so the agent's provider alone would leave it unpriced.
- Missing prices: a missing cache read or cache write price is the input
  price of the same prompt length (no discount guessed). A cache write
  never costs less than input: the file gives 0 for DeepSeek and Z.ai,
  which charge a write as plain input. A missing long output price is the
  standard output price. Of two long prompt limits the lowest is used.
- Rounding: dollars per token × 10¹² rounded to the nearest whole
  micro-dollar per million tokens; a negative or too large price leaves the
  entry out, like a missing input or output price.
- Before the first read the money of an API key profile has no estimate
  row in the sources; the dashes show in the Usage block.

Tests written first and seen failing: the LiteLLM reader (plain keys,
provider keys, cloud keys left out, long prompt prices, no cache prices,
two limits, broken files, rounding on real values; then the zero cache
write), the price source walk with a mocked request and a temp folder
(fetch and keep, no fetch within the day, fetch after a day, fetch again,
a failed or broken fetch keeps the last good file, 404, broken kept file),
the long prompt counting, `ListPrices`, the read order (start, on screen
only with an API key, Refresh after a failed fetch with and without a kept
file, no read after a good fetch within the day, a day old, on opening),
the dashes and the line, the statistics dialog with no prices, and a
broken file told once. The source rows (`model/sources.rs`) were written
before their code but could not be run red: the crate did not build until
the rest of the change was in. No test reaches the network: the io test
uses a kept copy written just now.

Run by hand (owner):

1. With the network on, an API key profile: "Where the numbers come from"
   shows "LiteLLM prices, updated <now>"; `~/.cache/otty/prices/litellm.json`
   exists. A restart within the day does not fetch again (same time shown).
2. Network off and `~/.cache/otty/prices` removed, start OTTY: the money
   rows show "—" with the line under them; the sources say why.
3. Network on, press Refresh: the money comes back.
4. A DeepSeek or new Claude model in the session files gets a price.
