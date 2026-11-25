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

## Comments

Built in `otty-agents` (2026-11-19):

- `provider.rs`: `Provider` (six providers), and the provider of a server
  address (`ANTHROPIC_BASE_URL`) by its domain; an unknown server has none.
- `key_fingerprint.rs`: `KeyFingerprint` (FNV-1a of 64 bits, so it is the
  same in every run) and the last four characters a key is shown by; the
  key is dropped while the file is read. A key shorter than 16 characters
  shows none of it.
- `account.rs`: `Account` (provider, `AccountIdentity`, `AccountLogin`,
  plan, `AccountKind`, `FoundIn` places) and `merge_accounts`: one account
  per provider and identity, with every place it was found.
- `Agent::accounts(profile)`. Claude Code (`claude_code/accounts.rs`, with
  `config_files.rs` now shared with `detect.rs`): the login (account id;
  a prepaid login with no plan is its organization, paid per token; the
  email when there is no id), `primaryApiKey`, and the settings key
  (`ANTHROPIC_AUTH_TOKEN`, else `ANTHROPIC_API_KEY`) at the provider of
  `ANTHROPIC_BASE_URL`. Codex (`codex/accounts.rs`): the ChatGPT login by
  account id (from `tokens` or the id token), email and plan from the id
  token, plan named as the limits name it ("Pro 200"); or the API key.
- Token records: `ModelTokens` carries `provider()` and `account()`, and
  the counts are kept apart by (provider, account, model). Claude Code
  takes the profile's provider (base URL, else Anthropic); Codex takes the
  first `session_meta` (`model_provider`, `openai` or none means OpenAI,
  any other id is unknown) and `creator_account_id`.

Decided differently: the login is an enum, not ready text, so the app
writes "key ••••a1f3" in the user's language. A ChatGPT login is always
kind Limits from its files; Credits is told by the limits answer
(issue 02). Status is left as it is: `docs/agents/triage-labels.md` has no
role for "done, awaiting the owner's check".
