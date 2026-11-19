# 09 Add account and admin keys

Status: ready-for-agent
Blocked by: 08

## Why

The last resort of spec Decision 7: a key no agent uses, and the admin key
of an organization bill.

## What to build

1. Add account dialog (Accounts "+"): provider (DeepSeek, OpenRouter,
   Moonshot; Anthropic or OpenAI organization), key file or environment
   variable, path or name. Saved in `agents.json` (path or name only).
2. Sources: DeepSeek `GET /user/balance` (Left), OpenRouter `GET
   /api/v1/key` (Key limit), Moonshot `GET /v1/users/me/balance` (Left),
   OpenAI `GET /v1/organization/costs` with an admin key (Spent this month),
   the existing Anthropic cost report. Same rules as today: OTTY's
   User-Agent, no redirects, 30 s timeout, the key read for the request and
   dropped.
3. Menu items: Admin key… on organization accounts; Edit key… and Remove on
   accounts added by hand (they also have Hide).

## Done when

- Tests first (mocked HTTP, made-up answers): each new source's answer and
  failure states; the saved entry has no key; Remove forgets it.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/` (new provider sources), feature core and io, i18n.

## Owner checks by hand

Add a DeepSeek key file; see Left; Remove it.

## Comments
- 2026-11-19, coordinator: issue 04 left the Accounts section without "+"; this issue adds it (Add account).
