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
