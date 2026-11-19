# 01 Accounts in the model

Status: ready-for-agent

## Why

Limits, money and bills belong to an account at a provider, not to a
profile (spec Decisions 2, 4, 5). Everything else in this spec builds on
these types.

## What to build

In `otty-agents`:

1. `Provider` (Anthropic, OpenAI, DeepSeek, OpenRouter, Moonshot, Kilo) and
   `Account`: provider, identity (account id, organization id, email, or a
   key fingerprint), login text to show ("you@example.com", "key ••••a1f3"),
   plan, kind (Limits, Credits, Money) and where it was found (profile
   folder, system keyring, added by the user).
2. `Agent` gains a way to list the accounts found in a profile (a new
   method, justified in its doc as the others are). Claude Code: the login
   of `.credentials.json` with `.claude.json` `oauthAccount` (id, org id,
   email, billing type), `primaryApiKey`, `env.ANTHROPIC_API_KEY` /
   `ANTHROPIC_AUTH_TOKEN` with `ANTHROPIC_BASE_URL` in `settings.json`.
   Codex: `auth.json` account id and the plan from the id token
   (`chatgpt_plan_type`).
3. A pure merge of the accounts of all profiles: one account per identity
   (spec Decision 2). Key fingerprints use a hash; the key itself is
   dropped right after hashing and is never kept or logged.
4. Token records carry (provider, model): Claude Code takes the provider of
   the profile's login or base URL; Codex the session's `model_provider`
   and its account id (`creator_account_id`) when present.

No UI change in this issue.

## Done when

- Tests written first and seen failing: Claude Code subscription, prepaid
  organization, API key in settings, DeepSeek base URL; Codex ChatGPT login
  with plan; the merge (same account in Codex and a second folder is one;
  two different ChatGPT logins stay two; same key in two places is one);
  no key or token in any `Debug` output.
- Made-up test data only.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (new `account.rs`, `provider.rs`; `agent.rs`;
`claude_code/detect.rs`, `codex/auth_file.rs`, the spend readers).

## Owner checks by hand

Nothing visible yet.
