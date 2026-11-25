# 11 OpenAI plugin, Codex part: logins and limits, embedded; core Codex deleted

Status: ready-for-agent
Blocked by: 05, 06, 08, 10

## Why

Spec Decisions 4, 5, 10, 11 and 13. The `codex` part must report its
login without seeing the token and read the limits Codex itself gives,
and the OpenAI plugin must ship inside the binary. Then the in-core Codex
code goes.

## What to build

1. Logins (`accounts.rs`, `auth_file.rs`, `base_url.rs`): from
   `auth.json` a ChatGPT login (provider `openai`, account id from
   `tokens.account_id`, email and plan from `jwt-claims` of the id token
   through a key handle; key place `/tokens/access_token`) or an API key
   (`/OPENAI_API_KEY`); a base URL in `config.toml` (`model_providers`)
   reported as the host the key is for. Key places declared in the
   `codex` part's permissions; nothing read as text.
2. Limits the agent gives: from session files (`session_limits.rs`,
   `rate_limits.rs` for the session records), and from `codex -s
   read-only -a never app-server` through `programs.talk`
   (`app_server.rs`), declared as a `run` of the `codex` part: only
   `initialize`, `initialized`, `account/rateLimits/read`. `thread/list`
   is never sent (spec Decision 13); a test fails if any other method
   name is written. Token expiry from claims (`token_expiry.rs`) gives
   "login expired".
3. The ChatGPT usage request (`codex/usage.rs`, `individual_limit.rs`,
   `workspace_credits.rs`) is not ported here: it is the provider part
   `openai` of the same plugin (issue 15). Move it in the core to
   `account_numbers/` so the Codex folder can go.
4. Embed: the built copy of the OpenAI plugin in
   `assets/plugins/openai/` with its hashes; installed on first start
   (issue 08). While the plugin lives in `plugins/openai/` (spec Decision
   10), the copy is built from there by a script (`plugins/embed.sh` or
   similar, used by issues 13, 14 and 16 too) and the issue comments say
   so.
5. Delete `crates/agents/src/codex/` (except what step 3 moved) and the
   built-in Codex registry entry in this issue; the owner checks the
   full `codex` part by hand (tokens, money, limits shown, the ChatGPT
   login joined with the same login found in another agent).

## Done when

- Tests first: the ported tests of `accounts.rs`, `auth_file.rs`,
  `base_url.rs`, `session_limits.rs`, `app_server.rs` (with a mock talk),
  `token_expiry.rs`; the plugin never gets the token text (test host
  check); the method list test.
- The core Codex folder is gone;
  the app shows Codex from the embedded OpenAI plugin with no network.
- The checks of `AGENTS.md` pass; the plugin's own tests pass.

## Owner checks by hand

Start OTTY with no network: Codex profiles, sessions, tokens and limits
appear; Profile info says "Read by: OpenAI <version>".

## Comments
- 2026-11-20, issue 10: the `codex` part already declares the `auth.json` key fields and has a small `auth_file.rs` (auth mode and whether a key or token is there) for `detect`; extend it for logins. The session's `creator_account_id` is not passed yet: an answer names its account only through `login`, so set it to the place of the login whose account id matches. The session list reads the first line of every session file on each call (no memory across calls, unlike the built-in reader); watch its time on the 10.8 GB folder.
- 2026-11-20, issue 10: the Codex session list reads the first line of every session file on each refresh (plugins keep no memory between calls). Measure on the real 10.8 GB folder; if slow, let the host remember unchanged files for `sessions` too, as it does for spend.
- 2026-11-20, issue 13: the embedding script is `plugins/embed.sh <folder>`; how to use it and add the `embedded_plugins()` entry is in the comments of issue 13.
- 2026-11-20, issue 11: done. Logins (`accounts.rs`, `auth_file.rs`, `base_url.rs` of the plugin): a ChatGPT login (`/tokens/access_token`, account id from `tokens.account_id` else the id token, email and plan from the id token's claims through its handle) or an API key (`/OPENAI_API_KEY`), as `auth_mode` says. Whom it is for: a ChatGPT login is for the host of `chatgpt_base_url` when that names a server that is not OpenAI's; the API key is for the host of the `[model_providers.<id>]` that `model_provider` picks when it has `requires_openai_auth = true`; else `openai`. A session's `creator_account_id` names the place of the profile's login with that account id (`read_profile_answers`).
- 2026-11-20, issue 11, limits: sources `app-server` (`codex -s read-only -a never app-server` through `programs.talk`, declared as a `run`; the only messages are the three of `app_server::Method`, and the tests check every line sent) then `session-files` (the last `rate_limits` record of the 10 newest session files, read through `files.lines` with the marker `"token_count"`). An API key login reads no limits (`not-supported`). An expired access token (its `exp` claim, `token_expiry.rs`) starts no app server, since Codex would renew the login while answering; the contract has no "login expired", so it is `key-refused` and the session files still answer.
- 2026-11-20, issue 11, the session list on the owner's real folder (23 GB today, 2 850 session files of which 2 038 `.zst`, 1 437 sessions): about 6.2 s per call, 98 % of it decoding the first line of each `.zst` file in the host. Two host changes bring it to about 4.4 s: the zstd decoder of one file is reset for the next instead of made anew, and a long line is searched for its newline once. Listing takes 20 ms, plain files 50 ms in all. It runs on each read of the sessions (every 5 minutes while Agents mode is open); the next step, if the owner wants it, is the host remembering unchanged files for `sessions` as it does for spend, which needs a contract change (per-file session reads, with the names of `session_index.jsonl` joined after).
- 2026-11-20, issue 11, embedded and deleted: `plugins/embed.sh openai` makes `assets/plugins/openai/` (`openai.wasm` about 380 KB); `embedded_plugins()` lists it. Deleted: `crates/agents/src/codex/` and `Codex`, and what only it used (`file_tree.rs`, `title.rs`, `jsonl.rs`, `read_if_changed`, `FileMemory::kept_or_read`, `read_seen_key`, `FileSpend::with_account`, `earliest`/`latest`, the test-only `RecordCounts` getters, the `ruzstd` dependency of `otty-agents`). The ChatGPT usage request moved to `account_numbers/chatgpt_usage.rs` with `chatgpt_limits.rs`, `chatgpt_individual_limit.rs`, `chatgpt_workspace_credits.rs`, `jwt.rs`, `token_expiry.rs`; it is still sent for a ChatGPT login no profile reads limits for, now with the workspace credits, and only to `chatgpt.com`. Tests use `otty_agents::fixtures::codex_stand_in()` where they need only Codex's name, commands and sources, the test agent `CodexLike` in `otty-agents`, and the embedded OpenAI plugin in the agents-io tests (`agents_with_plugins`). The plugins see an empty config folder as missing, so a test folder needs a file.
