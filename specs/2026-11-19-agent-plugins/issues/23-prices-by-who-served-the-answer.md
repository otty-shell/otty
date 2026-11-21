# 23 Prices: the plugin says who served an answer, OTTY picks the price

Status: ready-for-agent
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
