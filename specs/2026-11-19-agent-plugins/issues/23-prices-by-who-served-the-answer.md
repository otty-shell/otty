# 23 Prices: the plugin says who served an answer, OTTY picks the price

Status: done
Blocked by: 22

## Why

Owner, 2026-11-21. OTTY prices tokens with the LiteLLM price table,
fetched once for all plugins; that stays an OTTY feature. But the table
has several rows for one model, with different prices: the maker itself
(`claude-sonnet-4-5`), clouds (`bedrock/…`, `vertex_ai/…`, `azure/…`)
and resellers (`openrouter/anthropic/…`). Today OTTY picks the maker's
row through `MODEL_MAKERS` in `crates/agents/src/litellm.rs`, a list of
ten provider names in OTTY's code. That is against Decision 1 (no
provider names in OTTY), still lists the dropped Moonshot, and a new
maker needs an OTTY release to get prices.

Each answer already says who served it (`provider-ref`: a provider id
or the host of a server), but OTTY does not use it to pick a price.
And a part drops what the agent wrote when OTTY has no plugin for it:
OpenCode writes `providerID = "google"`, yet the OpenCode plugin
answers `None`, because its small map knows only the providers of the
official plugins. The name is useful for the price even without a
plugin.

## Who does what

- The plugin gives facts: the model as the provider names it, and who
  served the answer, read from the agent's settings and files (Claude
  Code: `ANTHROPIC_BASE_URL` or Anthropic; Codex: `base_url` or OpenAI;
  OpenCode: its `providerID`). It passes the provider's name as the
  agent wrote it, even when no plugin serves that provider.
- OTTY keeps the price table and one rule to pick a row:
  1. the row of who served the answer and the model;
  2. else the row of the model's maker;
  3. else no price: the money shows as "no price", never a guess.
- OTTY, not the plugin, matches a provider name to the plugin that
  serves it.

## What to build

1. First check the real LiteLLM table: is the maker's own price always
   the row with the plain model name (no `<provider>/` prefix), while
   clouds and resellers always have a prefix or another
   `litellm_provider`? Write what you found as a comment here. If yes,
   step 2 of the rule needs no list and `MODEL_MAKERS` goes.
2. Names: by default a provider's name in OTTY is the one the table
   uses. A provider part whose table name differs gives it in its
   manifest, in one optional field. Ask the owner if the check of step 1
   shows another way is simpler.
3. Who served the answer: let a part give a provider name that no
   installed plugin serves (the contract today takes only known ids or
   hosts; change it so the agent's own name passes through). OpenCode
   passes its `providerID` as written; OTTY joins it to a provider part
   when one serves it.
4. Pick the price by the rule above; remove `MODEL_MAKERS`.
5. Spec (Decision 6, the open question on `MODEL_MAKERS`) and docs
   (`Overview.mdx` on estimates, `Write a Plugin.mdx` on who served).

## Done when

- Tests first: an OpenCode answer served by OpenRouter gets OpenRouter's
  price; a Codex answer served by OpenAI gets OpenAI's; an answer with
  an unknown server gets the maker's price; a model with no row shows
  no price; an OpenCode `google` answer keeps its provider name and is
  priced without a Gemini plugin.
- No provider name left in OTTY's price code.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21, step 1, the check of the real LiteLLM table (downloaded
  that day: 7,091 rows, 4,889 with a price per token).
  - **Clouds and resellers: yes.** Every row of a cloud or a reseller
    either has a `<provider>/` prefix or another `litellm_provider`. Of
    4,238 rows with a prefix, 3,790 have the prefix equal to
    `litellm_provider` (`openrouter/…`, `azure/…`, `xai/…`); the other
    448 differ only by a suffix (`bedrock/…` is `bedrock_converse`,
    `vertex_ai/…` is `vertex_ai-anthropic_models` and the like). So the
    prefix names who sells a row, and OTTY needs no list for it.
  - **The maker's price is the plain name: only for some makers.** Of
    651 rows with a plain name, 179 are OpenAI's (155) and Anthropic's
    (24), and none of their models has a row only with a prefix: for
    them the plain name is always the maker's price. It fails in two
    ways:
    1. Six makers have no plain row at all, only `<maker>/<model>`:
       xAI (43 rows), Mistral (52), Z.ai (17), Moonshot (13), MiniMax
       (7), Alibaba's `dashscope` (27). Gemini has 11 plain rows and 31
       models only as `gemini/…`; DeepSeek 6 plain and 4 only as
       `deepseek/…`.
    2. Clouds use plain names too: 337 Bedrock rows and 44 Vertex rows.
       The Bedrock ones are Bedrock's own ids (`us.anthropic.claude-…`,
       `anthropic.claude-…-v1:0`; one exception,
       `claude-sonnet-4-5-20250929-v1:0`), never a name an agent writes
       for the maker. But the Vertex rows take Gemini's own names:
       `gemini-2.5-flash` is a Vertex row; 29 of them also have a
       `gemini/…` row, and 8 of those differ, only in a cache read price
       one side lacks (`gemini-3-pro-image`, `gemini-2.5-flash-image`).
  - Also found: `MODEL_MAKERS` listed `dashscope`, a reseller: it sells
    `dashscope/glm-5.1` and `dashscope/deepseek-v4-pro` too, and of two
    prefixed rows of one model the first by name won, so `glm-5.1` got
    Alibaba's price over Z.ai's (the same today, by chance). OpenRouter
    has only 21 rows of its own (`openrouter/anthropic/claude-fable-5.2`
    and the like); most models it serves have no OpenRouter row.
  - **What it means for the rule.** Step 2 (the maker's row) is the row
    with the plain name: right for OpenAI and Anthropic, close for
    Gemini (Vertex's list prices, the same but for 8 cache prices),
    and nothing for the six makers with prefixed rows only. Those are
    priced by step 1 when the answer says who served it, as OpenCode
    does (`xai`, `mistral`, `zai`, `minimax`); an answer of theirs with
    no or an unknown server shows "no price" instead of a guess. So
    `MODEL_MAKERS` goes, and no other list replaces it. A name the
    agent and the table write differently (OpenCode's `google` and
    `moonshotai`, the table's `gemini` and `moonshot`) gets step 1 only
    through a provider part that names the table's name in its manifest
    (item 2): `priced_as = "gemini"`.
- 2026-11-21, done. `MODEL_MAKERS` is gone; no provider name is left in
  OTTY's price code. `PriceTable::price_of(model, served_by)` follows the
  rule: the row `<served_by>/<model>`, else the plain row `<model>`,
  else no price (each with the dated snapshot fallback). The key's prefix
  says whose row it is, so the reader keeps every row and needs no list.
  `Registry::price_name_of` turns who served an answer into the table's
  name: the provider part's `priced_as` (a new optional manifest field),
  else its id; a host through the part that serves it; a name no part
  serves as written; an unknown host has none. The money of a session, a
  project, a day and an account takes the registry. The contract keeps
  `provider-ref` as it is; its docs now say a name passes even when no
  plugin serves it. OpenCode passes `providerID` as written for answers
  (its logins still map to the ids of OTTY's parts; issue 27 removes
  Kilo). Left for the owner: Codex still gives no provider for a
  `model_provider` other than `openai`, and Claude Code's
  `ANTHROPIC_BASE_URL` host is priced only when a part serves it, so a
  Z.ai or Moonshot server gets "no price" for GLM or Kimi models, which
  `MODEL_MAKERS` priced before.
