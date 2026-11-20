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
