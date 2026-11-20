# 15 Provider parts of the Anthropic and OpenAI plugins

Status: ready-for-agent
Blocked by: 05, 07, 11, 13

## Why

Spec Decisions 4, 7 and 10. The account numbers of Claude and ChatGPT
logins and of Anthropic and OpenAI organizations are provider work. After
issues 11 and 13 their code sits in `crates/agents/src/account_numbers/`.
They land as the provider parts of the two official plugins that already
hold the agent parts, so each plugin gets its second part in a new
release.

## What to build

1. In `plugins/anthropic/`, the provider part `anthropic` (hosts
   `anthropic.com`, `claude.ai`, usage pages for a login and for a key):
   `account` from a Claude login or a key (plan, organization);
   `numbers`: limits of a Claude login from `GET
   https://api.anthropic.com/api/oauth/usage` (`usage.rs`,
   `usage_answer.rs`); `bill`: the cost report with an admin key
   (`cost_report.rs`, pages walked in the plugin), `add_by_hand =
   "admin-key"`.
2. In `plugins/openai/`, the provider part `openai` (hosts `openai.com`,
   `chatgpt.com`): `account` from a ChatGPT login (claims) or a key;
   `numbers`: limits and credits of a ChatGPT login from
   `chatgpt.com/backend-api/wham/usage` (the code moved from
   `codex/usage.rs`, `rate_limits.rs`, `individual_limit.rs`,
   `workspace_credits.rs`), also when the login was found only in
   OpenCode; `bill`: `GET https://api.openai.com/v1/organization/costs`
   with an admin key (`openai_costs.rs`), `add_by_hand = "admin-key"`.
3. Each manifest gains a `[[provider]]` part with only its own hosts in
   its `http` permission, and `[api] provider = "0.1"`; each plugin's
   `export!` becomes `with agent, provider`; provider icons; made-up
   recorded answers as fixtures. Each plugin gets a new minor version,
   and the embedded copies in `assets/plugins/anthropic/` and
   `assets/plugins/openai/` are rebuilt.
4. Delete the moved files and the built-in entries; the owner checks
   the accounts' numbers by hand.

## Done when

- Tests first: the ported tests with the same made-up answers; a key
  handle goes only into a request to a declared host (test host check);
  while the `openai` part runs, reading the `codex` part's files is
  refused, and the same for `anthropic` and `claude-code`; a ChatGPT
  login found in OpenCode reads its limits once with the OpenCode key
  place.
- An installed plugin updated from the agent-only copy to this one shows
  the new provider part in the permission diff and waits for the user
  (issue 08); a first start with the rebuilt embedded copy installs both
  parts on.
- The core files are gone.
- The checks of `AGENTS.md` pass; the plugins' own tests pass.

## Comments
- 2026-11-20, issue 07 (left for the provider issues): provider parts get only accounts a plugin agent found. Still missing: a host key place for keys added by hand (a plain key file / variable from Add account) and for keys found only by built-in readers or in the keyring; `read-bill` for organizations with an admin key; `spent_this_month` and credits on key accounts (`AccountNumbers` holds only a Balance or Limits today); the names of a part's own limit sources (all show as one "Plugin" source).
- 2026-11-20, issue 12: the core usage request skipped expired tokens (`expiresAt`) and tokens without the `user:profile` scope; `login` / `login-facts` carry neither, and the provider part may not read `.credentials.json` — add them to the contract or just try and handle KeyRefused. A subscription login reports `account_id` and email, not its organization id: the provider part learns the organization from the API.
- 2026-11-20, issue 13: the Claude usage request is `account_numbers/claude_usage.rs` (+ `claude_usage_answer.rs`), already reduced to "token + plan → limits" and unused until this issue ports it; delete it then. Deleting `ClaudeCode` also removed the bill of a Claude Code profile's saved admin key (`Agent::bill_source`); organizations added by hand still read theirs through `account_numbers`. `anthropic_costs.rs` (the old `cost_report.rs`) is still shared with `openai_costs.rs` (`bill_of`, `day_index`).
- 2026-11-20, issue 11: the ChatGPT usage request is `account_numbers/chatgpt_usage.rs` (with `chatgpt_limits.rs`, `chatgpt_individual_limit.rs`, `chatgpt_workspace_credits.rs`, `jwt.rs`, `token_expiry.rs`), reduced to "access token + account id → limits and workspace credits" on `chatgpt.com`; the `codex` part has ported `plan_name`, `codex_limits` and the individual limit for its own sources, so the provider part can copy them from `plugins/openai/src/`. `chatgpt_base_url` is now told by the `codex` part as the host the login is for (a gateway is a `Host`, not `openai`), so a provider part that serves a gateway must take its URL from the host. The ChatGPT login of a Codex profile still reads through the built-in `openai` entry via `PluginAgent::account_key` until this issue.
- 2026-11-20, issue 14: a ChatGPT login found in OpenCode is a `subscription` login of provider `openai` with the account id, email and plan in its identity; its key place is `/openai/access` of `~/.local/share/opencode/auth.json`, or the `credential` field `/access` of `opencode.db` (row by rowid). OpenCode writes the access token only; the refresh token is a declared key field too but is never named as a place.
- 2026-11-20, issue 15: done. Both provider parts are in the plugins (Anthropic 0.2.0, OpenAI 0.2.0, embedded). Contract changes (WIT 0.1, in place): `account-numbers` gained `extra-usage` (Claude's extra usage), so every provider plugin was rebuilt; `limit-sources` keeps `list<string>` but each source is now named as the user reads it (`codex app-server`, `Codex session files`, `Claude Code /usage`), and that name is also the id `read-limits` gets. The manifest gained `plan_usage_page` (the page of a subscription; `usage_page` is the one of a key or an organization).
- 2026-11-20, issue 15: the question of issue 12 is settled as "try and handle KeyRefused": no expiry or scope in the contract. A refused subscription login shows as "not logged in", a refused key as "key refused"; the OpenAI part reads the `exp` claim of the ChatGPT token through its handle and sends nothing when it expired. Neither part learns an organization id from the API yet: `account` returns what the agent part found, so a Claude login is still joined by account id or email only.
- 2026-11-20, issue 15: key places: a key or admin key added by hand and a key found only in the keyring (after Allow) reach the part as `KeySource::Given`, read once for one read; an organization added by hand at a provider whose part says `add_by_hand = "admin-key"` reads `read-bill`. The admin key saved for an API key profile reads its bill again through the part of the provider of the profile's key (`read_bill(providers, provider, admin_key, …)`); `Agent::bill_source`, `Agent::has_bill` and `BillSource` are gone, the app asks the registry whether the profile's provider offers an admin key.
- 2026-11-20, issue 15: a key's numbers can now hold the money spent this month and credits (`Balance::spent_this_month`, `Balance::credits`), shown as rows of the Money block; the provider's own count of the month comes after a bill and before the estimate. Sources are named: a provider part's source is `SourceId::Named(<provider name>)` (its own reads and the first source of a profile's limits walk, before the agent part's sources); an agent part's sources are `SourceId::Named(<the part's name>)`, one read per source. `SourceId::Plugin` stays for sessions, statistics and spend.
