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
5. Delete `deepseek.rs`, `openrouter.rs` and their built-in entries;
   the owner checks the accounts by hand.

## Done when

- Tests first: the ported tests with the same made-up answers; a key
  added by hand (file or variable) reaches the plugin only as a handle;
  a DeepSeek key found behind Claude Code's `ANTHROPIC_BASE_URL` (host
  `api.deepseek.com`) is read by the DeepSeek plugin.
- The core files are gone; the app shows both from the
  embedded plugins.
- The checks of `AGENTS.md` pass; the plugins' own tests pass.

## Owner checks by hand

Before the embed, add each plugin with **Add plugin from folder…** and
pick it over the built-in reader; after, see both as "Official · built
in". Left of the DeepSeek account and Key limit of the OpenRouter key
match what the built-in reader showed.

## Comments
- 2026-11-20, issue 16: step 4 (embedding) is pending: issue 13 builds the shared embedding mechanism (`embedded.rs`, `assets/plugins/`, `plugins/embed.sh`) in parallel, and both plugins are embedded after it merges.
- 2026-11-20, issue 16: step 5 (deleting `deepseek.rs`, `openrouter.rs`, their built-in entries, `SourceId::DeepSeekBalance` / `OpenRouterKey` and their i18n keys) is left for the embedding step: it changes `account_numbers/mod.rs` and `built_in.rs`, which issue 13 changes in parallel, and without the embedded copies DeepSeek and OpenRouter accounts would have no reader on a first start.
- 2026-11-20, issue 16: keys added by hand now reach provider parts (the comment of issue 07 in issue 15). The host's key handle has a second source, `KeySource::Given`: OTTY reads the file or the variable the user gave (a variable through the login shell, as before) once for one read of the part (`account` and `numbers`) and drops it after that read, not before each request. An added account is read by a plugin part only when the part says `add_by_hand = "key"`; an added admin key still waits for `read-bill` (issue 15). Keys found only in the keyring are still not given to parts.
- 2026-11-20, issue 16: both icons are letter icons ("D", "O" on a rounded square), drawn as paths; no single-color logo with a licence that clearly allows shipping it was at hand.
