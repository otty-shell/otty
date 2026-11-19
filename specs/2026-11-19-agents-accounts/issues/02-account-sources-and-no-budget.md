# 02 Sources read per account, no budget

Status: ready-for-agent
Blocked by: 01

## Why

Two profiles of one ChatGPT account must cause one limits request, and the
bill belongs to the organization (spec Decision 5). OTTY only shows, so the
monthly budget goes.

## What to build

1. Limits, credits and bills are read per account: the existing sources
   (Anthropic usage API, Claude Code /usage probe, ChatGPT usage API, Codex
   app-server, latest Codex session, Anthropic cost report) are asked for an
   account, using the credentials of one profile where the account was
   found. Order and states stay as today.
2. Money of an account: from its bill when there is one, else the estimate
   over the tokens of the profiles that pay through it (spec Decision 4:
   not split when the agent does not tell accounts apart).
3. Remove the monthly budget: the field in the profile dialog, the budget
   bar, the model, and the `monthly_budget` key of `agents.json` (still read
   without error, then dropped on save).
4. The admin key moves from the profile to the account (kept in
   `agents.json`, keyed by the account identity). Migrate an existing
   profile admin key to the account of that profile.

## Done when

- Tests first: one request for two profiles of one account; the bill used
  for the organization account; the estimate summed over the linked
  profiles; a budget in an old `agents.json` read and dropped; the admin
  key migration.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (limits, bill, saved_profiles), `crates/app/features/agents/core` and `io` (reads, limits, bill, profile_dialog), i18n.

## Owner checks by hand

Codex and a second Claude folder logged in to one account: the account's
limits are read once (one "Updated" time). No budget anywhere.
