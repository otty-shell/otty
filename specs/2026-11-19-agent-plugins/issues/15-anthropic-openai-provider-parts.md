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
4. Parity: the numbers read by the core and by the plugin for the
   owner's accounts are the same (print only "equal" or the differing
   field names, never a value of a key); then delete the moved files and
   the built-in entries.

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
- Parity recorded; the core files are gone.
- The checks of `AGENTS.md` pass; the plugins' own tests pass.

## Comments
