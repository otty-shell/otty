# 12 Kilo and OpenCode

Status: needs-info
Blocked by: 01, 11

## Why

Spec Decision 10: Kilo (a fork of OpenCode) and OpenCode keep sessions in
SQLite with provider, model, tokens and cost on every message; the owner
uses Kilo.

## Needs the owner

A SQLite reader is a new dependency (no SQLite crate is in the workspace
today). Proposed: `rusqlite` with the `bundled` feature, opened read-only.
Approve before work starts.

## What to build

1. One reader for both: `kilo.db` / `opencode.db` (XDG data folder; `KILO_DB`,
   `OPENCODE_DB`), opened read-only so it survives the agent writing (WAL).
2. Profiles; sessions and projects; tokens (input, output, cache read, cache
   write, reasoning) and cost per message with provider and model.
3. Accounts: `auth.json`, `account.json` (Kilo; several per provider); Kilo
   account balance `/api/profile/balance` when logged in to Kilo.
4. Continue and resume commands (`kilo -s <id>`, `-c`; same for OpenCode),
   verified against the installed versions (Kilo 7.8.27, OpenCode 2.3.2).
5. Logos from the official repositories into `assets/svg` (licence noted).
6. Versions tested-up-to (issue 11).

## Done when

- Tests first on a made-up database: messages, tokens, cost, two OpenAI
  accounts not split, the local model without account.
- The checks of `AGENTS.md` pass.

## Owner checks by hand

Kilo shows its sessions, money from its own cost, and the ChatGPT account
shared with Codex.

## Comments

- 2026-11-19, owner: `rusqlite` with the `bundled` feature is approved; open databases read-only.
