# 37 Prices from the LiteLLM table, the bundled table as fallback

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
LiteLLM price table. It is a JSON file in the open source LiteLLM project,
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
   1. the fetched file;
   2. the kept file;
   3. the bundled table.
   - A model the fetched file has no price for, but the bundled table has,
     takes the bundled price. Say in the report whether you did this per
     model or per table, and why.
   - A broken or unexpected file is a failure of that source, told once,
     as other real failures are (issue 24); the walk goes on.
5. **On screen:** "Where the numbers come from" names the prices:
   - "LiteLLM prices, updated <date and time>";
   - or "Bundled prices of 16.11.2026" when the bundled table was used.
   - The tooltip says where the prices come from.
   - The money stays an estimate in its wording.
6. **The bundled table stays** as the fallback, unchanged except for what
   the new source needs. Keep its date.

## Done when

- Tests, written first and seen failing, cover:
  - reading a small hand-made file in the LiteLLM layout: plain keys,
    provider keys, cloud keys left out, long-prompt prices, a model with
    no cache prices, a broken file;
  - the float-to-micro-dollar rounding;
  - the fallback order (fetched, kept, bundled; per model as chosen);
  - fetching at most once a day;
  - the source name on screen.
- No test reaches the network.
- Run by hand: with network, the source says LiteLLM and a time. With the
  network off and the cache removed, it says bundled prices. A DeepSeek or
  new Claude model in the session files gets a price.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
