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

Built (2026-11-19):

- Dependency: `rusqlite` 0.43 (MIT) in `otty-agents`, feature `bundled`
  only (SQLite built from its C source by `libsqlite3-sys` 0.40, MIT; the
  default statement cache and WebAssembly build are left out). With it come
  `fallible-iterator` and `fallible-streaming-iterator` (MIT or
  Apache-2.0). `cargo deny check` is green.
- `otty-agents`, `open_code/`: one type `OpenCodeAgent` (`kilo()`,
  `opencode()`) for both, told apart by a `Fork` (names, data folder,
  database file, tested version: Kilo `7.8.27`, OpenCode `2.3.2`). The
  profile folder is the data folder, `$XDG_DATA_HOME/<name>` or
  `~/.local/share/<name>`; `KILO_DB` / `OPENCODE_DB` (absolute, or relative
  to the data folder, as the agents read them) move the database of the
  default profile. The app now reads each agent's database variable too
  (`AgentDescriptor::database_variable`).
- The database is opened `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`,
  `query_only`, a 2 s busy wait; nothing is migrated or written. Both
  shapes are read when there: `session` + `message` (Kilo 7, OpenCode 1)
  and `session_v2` + `session_message` (OpenCode 2.3, which no longer has
  `message`); a message in both counts once (its id). Tested: a write
  through OTTY's connection fails, and a read during the agent's open
  write transaction sees only what was committed.
- Sessions: top-level sessions with their folder and title (helper agents'
  sessions are not listed, their tokens count). Version: the `version` of
  the session that changed last. Continue: `kilo -s <id>`, new: `kilo`,
  log in: `kilo auth login` (same for `opencode`), checked in `kilo --help`
  7.8.27 and OpenCode's `packages/cli/src/commands/commands.ts` at
  v2.3.2. A profile in another folder named `kilo` starts with
  `XDG_DATA_HOME` = the folder above; a folder of another name can only
  get `KILO_DB` (its sessions there, the logins of the usual place).
- Tokens per (provider, model) of each answer: input + cache write as
  input (cache write also as `cache_written`), cache read as cached,
  output + reasoning as output (OpenAI counts reasoning in output). The
  provider id maps to a `Provider` (`openai`, `anthropic`, `deepseek`,
  `openrouter`, `moonshotai`, `kilo`); another id, such as a local model,
  has none. The cost the agent wrote is kept on each answer
  (`ModelTokens::cost`, `CountedAnswer::cost`); `MoneyTotal` uses it before
  the list prices and tells `is_estimate()`. Money of these profiles has
  no "~" (left panel, account page, usage block) and its source row is
  "Cost written by the agent"; tokens and sessions come from "Agent's
  database" (new `SourceId`s, 4 i18n keys in 10 languages).
- Accounts: `auth.json`, Kilo's `account.json` (several logins of a
  provider) and the `credential` table. A ChatGPT login is told by its
  account id (field, `metadata.accountID`, or the access token), with the
  email and plan of its access token, so it merges with Codex's login of
  the same id. A key, and a Kilo login token (a key of the Kilo gateway),
  is a key account by fingerprint. Two ChatGPT logins in one profile turn
  By account off ("uses 2 OpenAI accounts") by the existing rule.
- Format changed: each assistant message is a record; renamed tokens in
  the newest answers fail the count as issue 11 built, with the database
  and the version in the report.
- Statistics: session (with its helper sessions; messages = the user's)
  and project, over the whole history.
- Logos: `assets/svg/agent-kilo.svg` (Kilo-Org/kilocode v7.8.27,
  `packages/kilo-docs/public/favicon/favicon.svg`, MIT) and
  `agent-opencode.svg` (anomalyco/opencode v2.3.2,
  `packages/identity/mark.svg`, MIT), one color, source and licence in
  each file. The Kilo provider uses the Kilo mark.
- Live read of the owner's Kilo 7.8.27 (read-only, counts only): 2 OpenAI
  accounts (one shared with Codex, so 2 accounts in all with Codex),
  97 listed sessions of 331, newest session version 7.5.9, 6 573 answers
  and 577 497 255 tokens over the whole history, the same as a direct SQL
  count; no tokens in the last 30 days. OpenCode 2.3.2: no logins or
  sessions yet.

Decided differently:

- The icons are drawn in `#000` like the other agent logos, which the app
  tints, not `currentColor`. OpenCode's inner square is half transparent
  to keep the mark readable in one color.
- Money still needs the price table read, as for the other agents: the
  cost the agent wrote is used for every answer that has one. Kilo writes
  `0` for answers paid by a ChatGPT login; such a profile has Money off
  (subscription) as before.
- No limits are read through Kilo or OpenCode: a ChatGPT login shared with
  Codex gets its limits through Codex; a login found only in Kilo shows no
  limits yet. A Claude login of OpenCode has no id to tell it apart and is
  no account.
- Kilo's balance (`/api/profile/balance`) is moved to issue 09 with the
  other money sources.
