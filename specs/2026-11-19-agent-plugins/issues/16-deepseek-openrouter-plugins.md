# 16 DeepSeek and OpenRouter plugins

Status: ready-for-agent
Blocked by: 05, 07, 09

## Why

Spec Decision 10. Each of these is one request with a key and a few
numbers in JSON; together they finish the official providers. Each is a
plugin with one provider part and no agent part. Kilo Gateway is not in
the official set (owner, 2026-11-20); its code goes in issue 17.

## What to build

1. `plugins/deepseek/` (plugin id `deepseek`; provider part `deepseek`,
   host `deepseek.com`, `add_by_hand = "key"`): Left from `GET
   https://api.deepseek.com/user/balance` (`account_numbers/deepseek.rs`).
2. `plugins/openrouter/` (plugin id `openrouter`; provider part
   `openrouter`, host `openrouter.ai`, `add_by_hand = "key"`): Key limit
   and use from `GET https://openrouter.ai/api/v1/key` (`openrouter.rs`).
3. Each: own cargo workspace, `README.md`, manifest with `[api] provider
   = "0.1"` and `export!(<Name> with provider)`, its `http` permission
   with only its API host, icon (a letter icon is fine if no official
   single-color logo with a usable licence exists; note the choice),
   made-up answers.
4. Embed both: built copies in `assets/plugins/deepseek/` and
   `assets/plugins/openrouter/` with their hashes, by the script of issue
   11; installed on first start with their parts on (issue 08). With no
   DeepSeek or OpenRouter account found they do nothing.
5. Parity on the owner's accounts ("equal" or differing field names
   only); delete `deepseek.rs`, `openrouter.rs` and their built-in
   entries.

## Done when

- Tests first: the ported tests with the same made-up answers; a key
  added by hand (file or variable) reaches the plugin only as a handle;
  a DeepSeek key found behind Claude Code's `ANTHROPIC_BASE_URL` (host
  `api.deepseek.com`) is read by the DeepSeek plugin.
- Parity recorded; the core files are gone; the app shows both from the
  embedded plugins.
- The checks of `AGENTS.md` pass; the plugins' own tests pass.

## Owner checks by hand

Before the embed, add each plugin with **Add plugin from folder…** and
pick it over the built-in reader; after, see both as "Official · built
in". Left of the DeepSeek account and Key limit of the OpenRouter key
match what the built-in reader showed.

## Comments
