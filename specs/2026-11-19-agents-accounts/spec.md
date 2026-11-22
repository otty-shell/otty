# Agents mode: accounts apart from profiles

Status: decisions agreed with the owner on 2026-11-19

## Problem

Agents mode (spec `2026-11-16-agents-mode`) treats one profile as one
agent folder, one login and one provider. That no longer holds:

- most agents work with several providers, and Claude Code and Codex can
  be pointed at others (`ANTHROPIC_BASE_URL`, `model_providers`);
- one login is used by several agents (on the owner's machine one ChatGPT
  account is logged in to both Codex and Kilo);
- limits, money left and bills belong to a login or a key at a provider,
  not to an agent folder, so today the same limits can be read twice.

The research `docs/research/coding-agents.md` (17 agents, primary sources,
2026-11-19) backs these points.

## Goal

The user sees two things side by side and never has to type what OTTY can
find:

1. **Accounts**: where they pay. Subscriptions and their limits, credits,
   keys and the money left or spent on them.
2. **Profiles**: what they work with. Agent programs and their folders,
   sessions, projects and the tokens and money they used.

OTTY finds agents, profiles and accounts by itself in the agents' folders.
The user acts only where nothing else works: allow the system keyring, add
an admin key, add a key no agent uses, log in again in an agent.

The design is the prototype on the website, approved by the owner on
2026-11-19: `website/src/lite/prototype-accounts/` (page
`/lite/prototype-accounts/`, params `situation`, `select`, `menu`, `ctx`,
`open`, `added`, `w` are documented in the top comment of
`AccountsPrototype.tsx`). On the look the prototype wins; on behaviour this
spec wins. The prototype's data is made up.

## Words

- **Agent**: a coding agent program run in a terminal (CLI), such as Claude
  Code, Codex, Kilo, OpenCode, Goose, Kimi Code.
- **Profile**: one agent with one folder (or set of folders) that holds its
  sessions and logins, with a name. `~/.claude` and `~/.claude-work` are two
  profiles of Claude Code.
- **Provider**: who sells the model use: Anthropic, OpenAI, DeepSeek,
  OpenRouter, Moonshot, Kilo. Local models have no provider account.
- **Account**: one login or key at a provider: a Claude Max login, a
  ChatGPT login, an Anthropic organization, a DeepSeek key. It owns limits,
  credits, money left and bills.
- **Source**: one way to get one number, as today (spec 2026-11-16,
  Decision 5): ask a server, run the agent, read a file.
- **Need**: a task only the user can do, with one button: allow the
  keyring, add an admin key, log in again in an agent, log in to an agent,
  copy a report of a changed format.

## Decisions

### 1. Two sections, two kinds of pages

The left panel has **Accounts**, **Profiles** and **Projects**.

- Accounts starts with **All accounts**; Profiles starts with **All
  profiles**. Each list row shows its number with its word ("62 % used",
  "¥110 left", "~ $42.10 this month", "30.6M today", "no sessions", "log
  in", "format changed", "login expired", "hidden"; "—" only when there is
  nothing).
- **Projects keeps its search and its New session ("+") exactly as the app
  has them today.**
- Accounts "+" opens **Add account** (Decision 7). Profiles "+" opens the
  existing Add profile dialog.

### 2. OTTY finds everything it can

- Profiles: the default folders of each known agent, plus extra folders of
  the same agent found in the home folder when they hold its login or
  sessions (`~/.claude*`, `~/.codex*` today). Profiles the user added stay
  as today.
- Accounts: from each profile's files: Claude Code `.credentials.json` and
  `.claude.json` (`oauthAccount`, `primaryApiKey`), `settings.json` `env`
  keys; Codex `auth.json` (account id, plan in the id token); Kilo and
  OpenCode `auth.json`, `account.json` and their database; Goose
  `chatgpt_codex/tokens.json` and its keyring entries; Kimi Code
  `credentials/*.json` and `config.toml` keys.
- **One account, one row:** logins with the same account id (or
  organization id, or email for a login without an id) are one account;
  keys are the same account when their fingerprint (a hash, kept in memory
  only) is the same. OTTY never stores a key or a token.
- On the first start a **Found by OTTY** dialog lists the agents (version,
  folder, accounts), the accounts (where found, what OTTY reads, the number
  now) and the needs, split into "Accounts need you" and "Profiles need
  you".

### 3. Never write or renew another agent's login

OTTY only reads the agents' files, opens their databases read-only, and
never refreshes a token: refreshing can log the agent out. An expired login
is "login expired" with the need "Open <agent>" (opens the agent in a
terminal tab). After the user logs in there, OTTY picks the new login up.

### 4. Tokens belong to profiles, attributed per provider and model

- Each token record carries (provider, model). When the agent writes the
  provider (Kilo, OpenCode, Codex per session), it is used; when it does
  not (Claude Code), the provider of the profile's login or base URL is
  used.
- Tokens are tied to an account only when the agent writes it (Codex per
  session) or the profile has one account of that provider. When a profile
  uses several accounts of one provider and does not write which one
  served a message (Kilo with two ChatGPT logins), its tokens are **not
  split**: the account page shows the profile with "uses 2 OpenAI accounts"
  and no number.

### 5. Account numbers: Limits, Credits, Money

- **Limits** (subscriptions): windows with "N % used" and reset time.
- **Credits** (e.g. ChatGPT Enterprise usage-based): credits used of the
  limit.
- **Money** (keys, organizations): rows **Left** (money left on the
  account), **Spent this month** (from the bill, or an estimate marked
  "~"), **Key limit** (OpenRouter).
- **No budget.** OTTY only shows; the monthly budget is removed from the
  model, the dialogs and `agents.json` (read and ignored when present).
- Sources are read **once per account**, not per profile: two profiles of
  one ChatGPT account cause one request.

### 6. Pages

- **All accounts**: table Account, Provider, Now; **Needs you** with the
  account needs only (to-do list: title, line, one button; rows split by
  the table rule, no boxes).
- **All profiles**: table Profile, Agent, Version, Today; **Needs you**
  with the profile needs only.
- **Account page**: Needs from you (when any); the Limits/Credits/Money
  block; **Used by** chips (agent icon, profile, tokens today through this
  provider, or "uses 2 OpenAI accounts", or "no tokens yet"); **Account
  info** (Provider, Email or Key, Plan, Found in = the agents' folders, or
  "system keyring", or "added by you"); Where the numbers come from.
- **Profile page**: Needs from you; an info line ("no sessions yet",
  "installed, not used yet"); the **usage block**; **Pays through** chips;
  **Profile info** (Agent, Version, Folder, Projects, Sessions); Where the
  numbers come from.
- **Usage block**: dropdowns at the right of its title, period (Today, 7
  days, 30 days) and measure (Tokens, Money); a chart in the look of the
  kit `DayBars` (muted bars, today or the current hour in the accent; the
  hovered or picked table row lights its part of every bar in the accent);
  tabs above the table: By project | By model | By account. Options that
  are not possible are disabled with their reason: Money for a
  subscription ("Paid by a subscription: there is no price per request"),
  By account when the agent does not tell accounts apart. Defaults: By
  model, 7 days, Tokens. It must not overflow at a narrow main area.
- **Project page**: Project info (Path, Profiles chips, Accounts chips,
  Sessions, First and Last activity); the sessions table of all profiles,
  newest first, with a Profile column (agent icon, link), and today's
  interactions kept: click selects, double click and Enter continue,
  right-click and the row "⋯" have Continue, Continue in a split, Copy
  resume command, Copy session ID, Show statistics; search; sort by any
  column. **Project statistics** stays a button that opens a popup: facts
  and the usage block with tabs By profile | By model | By account.
  **Session statistics** stays a popup from the session menu: facts, an
  Account line (written in the session, the profile's account, or "one of
  2 OpenAI accounts") and the usage block with By model | By account. No
  "By session" tab anywhere.

### 7. The user's own choices

- "⋯" on a page and a right click on a row open the same menu.
  - Account: Rename…, Open usage page; Admin key… for an organization;
    Edit key… and Remove for an account added by hand; Hide or Show again.
  - Profile: Rename…, Open folder, Hide or Show again.
  - All accounts / All profiles "⋯": Show hidden … (N) / Hide hidden ….
- A hidden account or profile is not read; it leaves lists, chips and
  needs until Show hidden is on. An account two plugins read is two
  copies, one per plugin, and each is hidden alone, an added account too
  (spec 2026-11-19-agent-plugins, issues 29 and 31).
- **Add account** is the last resort: a provider (DeepSeek, OpenRouter,
  Moonshot money; Anthropic or OpenAI organization bill with an admin key),
  and a key file or an environment variable. OTTY saves only the path or
  the variable name.
- Names, hidden items, added accounts and admin key paths are saved in
  `~/.config/otty/agents.json`.

### 8. The system keyring, with permission

Some agents keep keys in the system keyring (Goose by default, Copilot).
OTTY reads only the entries of known agents, and only after the user
allows it once (need "System keyring · Allow OTTY to read it"). A new
dependency is approved by the owner for this (2026-11-19): Linux Secret
Service and macOS Keychain.

### 9. Agents change their files; OTTY says so

- Each reader counts the records it saw and the ones it understood. When
  new records arrive but none or far fewer are understood, the profile's
  state is **format changed** instead of quietly showing fewer tokens.
- OTTY knows each agent's version (from its records or files) and the
  newest version it was tested with. A newer version is still read; when
  reading degrades, the need names the version: "Claude Code 2.2.0 changed
  its session files. This OTTY reads them up to 2.1.x."
- The last good numbers stay, greyed, with their time; other sources,
  profiles and accounts work as usual.
- **Copy report** copies agent, version, file kind and what was not
  understood, with no keys, tokens or texts of sessions.

### 10. Which agents

CLI agents only. In this spec: Claude Code and Codex (as today, reworked),
Kilo and OpenCode (one reader: Kilo is a fork of OpenCode, same database),
Goose, Kimi Code (after the owner's first real session confirms its
files). Later: Crush. Out of scope: Antigravity CLI, Cursor, Copilot CLI,
desktop agents.

## Out of scope

- Splitting one agent's tokens between several accounts of one provider
  when the agent does not record the account.
- Any control over spending (limits, budgets, alerts).
- Logging in, out or renewing a login on behalf of an agent.
- Windows.
- Agents named out of scope in Decision 10.

## Docs

`docs/public/Agents/` is updated with the work: the Overview explains
accounts and profiles, and each provider and agent has a page listing every
file OTTY reads, every request it sends and to whom, including requests
sent with logins found in another agent (e.g. a ChatGPT login found in
Kilo).
