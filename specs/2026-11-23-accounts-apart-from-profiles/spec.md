# Agents mode: accounts apart from profiles

Status: decisions agreed with the owner on 2026-11-23

This spec changes Agents mode of spec `2026-11-19-agents-accounts` and the
plugin contracts of spec `2026-11-19-agent-plugins`. Where they differ,
this spec wins. The research behind it is
`specs/2026-11-19-agent-plugins/research/opencode-chatgpt-login.md`.

## Problem

Today an agent part tells OTTY which logins sit in a profile, OTTY joins
the logins of all plugins into accounts, and then links both ways: a
profile "Pays through" accounts, an account is "Used by" profiles, and
tokens are split "By account". This breaks in real cases:

- **A login may hide who it is.** OpenCode's ChatGPT login
  (`chatgpt-token-sharing`) keeps its account and email encrypted. A probe
  with a real token on 2026-11-23 showed that `chatgpt.com/backend-api/wham/usage`
  and `auth.openai.com/.../whoami` refuse it, and `userinfo` gives only an
  opaque `sub` (a per-client user id). So OTTY can show no limits, no email
  and no plan for it, and can not tell whether it is the same account as a
  Codex login. Issue 34 named such accounts "login", and issue 43 planned
  to keep them by `sub`.
- **Joining logins across plugins is guesswork.** Two plugins that read
  one provider make two copies of an account (issues 29 and 31), which the
  user hides one by one.
- **The links mislead.** A profile's tokens can not be split between two
  accounts of one provider when the agent does not write which one served
  an answer, so "Used by", "By account" and the key spend estimate are
  often empty or wrong.
- **An account needs a working profile.** To see a Claude balance the
  user must keep a Claude Code profile that they may not use.

## Goal

Agents mode shows two things that do not know about each other:

1. **Profiles**: what the user works with. Agent folders, their projects,
   sessions, tokens, and what those tokens would cost at API prices.
2. **Accounts**: where the user pays. Subscriptions, keys and
   organizations, with their limits, credits, money and bills, as the
   provider tells them.

A plugin fills either side or both, and OTTY draws no line between them.

## Words

- **Profile**: one agent with one config folder that the user works with,
  such as Codex in `~/.codex`. It has sessions, projects and tokens, and is
  listed under Profiles.
- **Hidden profile**: a config folder that OTTY makes for itself to keep
  its own login of an agent, under OTTY's data folder, such as
  `~/.local/share/otty/accounts/<id>/`. Only the provider part of the
  plugin that made it reads it. It never shows as a profile, and its
  projects and sessions never show anywhere.
- **Login**: what an agent keeps after the user logs in: tokens or a key
  in its folder (or in the system keychain on macOS).
- **Account**: where the user pays at a provider: a subscription (a
  ChatGPT or Claude plan), a key, or an organization. A provider part
  finds it from a login and reads its numbers: limits, credits, money,
  bill.
- **Slot**: one of the two places a plugin can fill in Agents mode. The
  **agent slot** fills Profiles and Projects, the **provider slot** fills
  Accounts. An agent part fills the first, a provider part the second.
  (Not the `slot` of an `ask-key` need, which keeps its meaning.)

Other words (plugin, part, need, key handle, registry) keep their meaning
from spec `2026-11-19-agent-plugins`.

## Decisions

### 1. Two independent slots, no link between them

Agents mode has an agent slot and a provider slot. Nothing links a
profile to an account: no "Pays through", no "Used by", no tokens by
account. OTTY merges nothing across plugins.

Why (owner, 2026-11-23): OTTY can not know for sure which account paid for
a profile's tokens, and for some logins (OpenCode's ChatGPT login) it can
not even know whose login it is. A link that is often missing or wrong is
worse than no link. The user wants to see what they work with and where
they pay, and both stand on their own.

### 2. The agent part does agent things only

An agent part gives profiles, projects, sessions and tokens, and the
launch and resume commands. It knows nothing about accounts: `logins`,
`login-refused`, the `login` of an answer and the `connection` of a
profile leave the agent contract. It keeps the `not-logged-in` status
with its need ("No login", "Login expired", and the command that fixes
it): whether its own agent can log in is the agent's business, not whose
account it is (Decision 9).

Why: the agent's job is its own files. Logins were in the agent contract
only to feed the links of Decision 1.

### 3. Money of a profile is always at API prices

A profile's money comes from its agent part. When the agent writes the
cost itself (OpenCode writes a cost per answer), the part gives that cost
as it is, with no mark. Otherwise the part asks OTTY for the price of a
model (a host call, OTTY answers from the price list it keeps and renews,
LiteLLM today) and prices the tokens with it, and that money is marked
**"at API prices"**, which replaces the mark "estimate, list prices". This
holds on a subscription too. Money is never disabled in the usage block.

Owner, 2026-11-23: "we ask the plugin for the price of the tokens. If it
can give it on the spot without asking the price list, let it give it. If
it can not, let it ask OTTY for the price, OTTY gives it from the price
list it keeps and renews."

Why (owner): on a subscription the number still answers a real question,
"what would this work cost me on the API", and it needs no account. The
mark keeps it honest.

### 4. OTTY finds only the default profiles

When an agent part is on, OTTY shows its default profile (the folder the
agent uses with no variable set, or the one its variable names, such as
`~/.codex` or `~/.claude`). The user adds other profiles with Add profile,
as today. OTTY no longer scans the home folder for `~/.codex*` or
`~/.claude*`.

Why: a found folder was often an old copy or another tool's, and the scan
existed mostly to find more logins. With accounts apart, the user adds a
second profile when they work with it.

### 5. The provider part owns accounts, from logins of its own plugin

A provider part finds accounts in two places:

1. **A profile of the same plugin.** OTTY gives the provider part the
   folders of the profiles of the agent parts its manifest names (the
   OpenAI part gets the Codex profiles). The part reads the login there
   itself, with its own declared permissions, and gets keys only as
   handles (Decision 5 of spec `2026-11-19-agent-plugins` stays).
2. **Its own hidden profile** (Decision 6).

A provider part never sees the profiles of another plugin.

Why (owner): the plugin that knows an agent's files also knows how its
login works and whose it is. Reading another plugin's logins is what made
copies (issues 29 and 31) and nameless accounts (OpenCode).

### 6. Add account: "Log in with <agent>" makes a hidden profile

Add account lists, for each provider part that offers it, **Log in with
<agent>** (such as "Log in with Codex", "Log in with Claude Code"). OTTY
then:

1. Makes an empty folder `<data dir>/accounts/<id>/`, where `<id>` is a
   new random id.
2. Runs the agent's official login command in a terminal tab, pointed at
   that folder: `codex login` with `CODEX_HOME=<folder>`, `claude login`
   with `CLAUDE_CONFIG_DIR=<folder>` (the provider part gives the command).
3. When the command ends, asks the provider part for accounts in that
   folder. An account found there is shown. No account found removes the
   folder and says "The login did not finish".

The hidden profile belongs to OTTY. It never shows as a profile, and its
projects and sessions (such as the session files of a `/usage` probe) are
never read or shown.

Why (owner): a user who wants to see a Claude balance should not need a
Claude Code profile they work in. CodexBar does the same for Codex with
its `managed-codex-homes/<uuid>/` folders. Using the agent's own login
command means OTTY never handles a password or an OAuth flow itself.

### 7. The agent program keeps OTTY's own login fresh

OTTY never refreshes a token itself, in any folder. In a hidden profile
the provider part runs the agent program there to read numbers, and the
agent renews its own tokens as it always does: `codex app-server` with
`CODEX_HOME=<folder>` (as CodexBar does), or the `claude` `/usage` probe
with `CLAUDE_CONFIG_DIR=<folder>`.

Why: renewing a token by hand can rotate it away from the agent and log
the agent out (Decision 3 of spec `2026-11-19-agents-accounts`). Letting
the agent do it in a folder only OTTY uses keeps that rule.

### 8. What the plugin tells about an account

For each account, the provider part tells:

- A **stable id**, unique within the plugin. OTTY keeps the user's name
  and Hide by plugin id + account id.
- A **display name**: an email, an email and a workspace, or a short key
  ("key ••••a1f3").
- A **kind**: subscription, key, or organization.
- **Where it was found**: profile folders and own hidden profiles.
- **Numbers**: limits, credits, money (left, spent this month, key
  limit), and the bill of an organization.
- **Needs**, such as "Login expired" with its fix command
  (`run(codex login)` with that profile's `CODEX_HOME`), or for a hidden
  profile "Log in again".

Whether two logins are one account is decided inside the plugin: one
ChatGPT account logged in to `~/.codex` and to a hidden profile is one
account with two places. OTTY does not join accounts of different plugins
or different provider parts.

Why: only the plugin knows what makes two logins the same at its
provider. A stable id the plugin chooses keeps names and Hide across
token refreshes, which a key fingerprint could not (the reason for issue
43).

### 9. A login need shows wherever it is met, and is fixed from any of them

A login need ("No login", "Login expired", with its fix command such as
`codex login` in that folder) is shown on every page it concerns, and the
user may act from any of them; once fixed it goes from all of them.

- **A profile with no login.** The agent part says so, as today's "not
  logged in" ("No login in `~/.codex-work` [Run login]"), on the profile
  page, as a mark on its row and in All profiles → Needs you. There is no
  account, so nothing shows in Accounts. Once the user logs in, the need
  goes and the provider part finds the account. An agent that works with
  no login (local models, OpenCode's free models) reports no such need:
  the plugin decides.
- **A profile whose login expired.** Whether a login expired is the
  plugin's call, by whatever means it has: the expiry written in the
  files, or a request to its server with its own permissions. OTTY does
  not work it out. The agent part shows the need on the profile, and the
  provider part shows it on the account found in that profile ("Login
  expired [Run login]"). OTTY does not copy one to the other: each part of
  the plugin says it for its own page. Both run the same command in the
  same folder, so acting in either place fixes both, and both go at the
  next read.

  Owner, 2026-11-23: "can the plugin not see that the login expired? Why
  is it OTTY's job?"
- **An account with no profile** (OTTY's own login in a hidden profile).
  The need shows only on the account, as "Log in again".

Why (owner, 2026-11-23): "if a profile is found and its account is not
logged in, both get this need from you, and I can act in either place";
"it may also show on an account, for example one I added with no
profile". Showing a profile's missing login in Accounts would not be
understood: there is no account yet. The profile also keeps its own needs
(format changed, folder not found, agent missing).

### 10. Accounts added by hand are removed, "paste a key" comes later

The old Add account ("a provider and the place of its key, a file or an
environment variable") is removed, with Edit key… and the `ask-key` need
with no slot that replaced a refused key of such an account (issue 26).
"Paste a key", with the key kept in the system keyring, comes later (see
Later). An `ask-key` need with a slot, such as the admin key of an
Anthropic or OpenAI bill, stays as it is.

Why (owner): an account known only by where its key lies can not be told
apart from a copy of itself and was the last resort nobody used. Pasting a
key is simpler for the user and is planned with keyring storage.

### 11. The official plugins

| Plugin | Agent part | Provider part |
| --- | --- | --- |
| OpenAI | Codex | OpenAI: reads Codex logins and its own hidden Codex profiles |
| Anthropic | Claude Code | Anthropic: reads Claude Code logins, the `/usage` probe, its own hidden Claude Code profiles |
| OpenCode | OpenCode | none. Its login code goes |
| DeepSeek | none | DeepSeek: no accounts until "paste a key" |
| OpenRouter | none | OpenRouter: no accounts until "paste a key" |
| template | an example agent | an example provider |

Why: OpenCode keeps logins for many providers, but the probe showed its
ChatGPT login tells nothing about the account. Its tokens and money at API
prices are what it can show well.

### 12. No backward compatibility

Agents mode was never released. `agents.json`, the WIT contracts
(`otty:agents-agent@0.1.0`, `otty:agents-provider@0.1.0`) and the
official plugins change in place. No migration, no version bump, no
reader of the old shapes.

Why (owner): Agents mode was never released, so nobody has old files or
plugins to keep working.

## Scenarios

The owner's table:

| What you have | What you see |
| --- | --- |
| Codex in `~/.codex` | Profile Codex with sessions. Separately a ChatGPT account with limits |
| A second Codex profile with another account | A second profile and a second account |
| OpenCode | Only a profile with sessions and tokens. No account |
| Want Claude balance without a working profile | Add account → Log in with Claude Code, an account with a hidden profile |
| An OpenRouter key | Later, through "paste a key" |

Two more that follow from the decisions:

| What you have | What you see |
| --- | --- |
| One ChatGPT account in `~/.codex` and in a hidden profile | One account, "Found in ~/.codex" and "OTTY's own login" |
| Claude Code pointed at DeepSeek in its `settings.json` | The profile's tokens priced as DeepSeek models at API prices. No DeepSeek account until "paste a key" |

## Before → after

### Left panel

- Before: Accounts (logo + name), Profiles, Projects. Profiles included
  folders OTTY found in the home folder. Money in rows marked "estimate,
  list prices".
- After: the same three sections and look. Accounts: logo + name as
  today, one row per account of each provider part. Profiles: default
  profiles and the ones the user added, never a hidden profile. Money
  marked "at API prices".

### Profile page

- Before: Needs from you (with "Run login"), usage block, **Pays through**
  chips, Profile info, Read by.
- After: Needs from you (profile needs, with "No login" and "Login
  expired" when the agent part says so, Decision 9), usage block, Profile
  info (Agent, Version, Folder, Projects, Sessions), Read by. No Pays
  through.

### Project page

- Before: Project info with Profiles chips and **Accounts** chips,
  sessions table with an **Account** column.
- After: Project info with Profiles chips only, sessions table without
  the Account column. Everything else as today.

### Account page

- Before: Needs from you, Limits/Credits/Money, **Used by** chips, Account
  info with "Found in" (agent folders, or "added by you"), menu with Edit
  key… and Remove for an account added by hand.
- After: Needs from you (with the login needs), Limits/Credits/Money and
  the bill as the provider tells them, Account info: Provider, Name, Kind,
  Plan, **Source** ("Found in ~/.codex", "OTTY's own login", or both), Read
  by. Menu: Rename…, Hide, Open usage page, and for its own login **Log in
  again** and **Remove**. Remove deletes the hidden profile folder after a
  question. The account leaves only when no other place holds its login.
  No Used by.

### All accounts

- Before: table Account, Provider, Now. Now could be a spend estimate from
  the tokens of the profiles that pay through a key.
- After: same table. Now shows only what the provider part read (percent
  used, money left, spent this month), else "—". Needs you holds the
  login needs.

### All profiles

- Before: table Profile, Agent, Version, Today, with found extra folders.
- After: same table with default and added profiles only. Needs you holds
  the profiles' needs, login needs included (Decision 9).

### Usage block and statistics popup

- Before: tabs By project | By model | **By account**. Money disabled on a
  subscription ("Paid by a subscription: there is no price per request").
  Session statistics had an **Account** line.
- After: tabs By project | By model on the profile page, By profile | By
  model in Project statistics, By model in Session statistics. Money is
  always on and marked "at API prices". No Account line.

### Add account dialog

- Before: pick a provider, then a key file or an environment variable.
- After: a list of "Log in with <agent>" entries, one per provider part
  that offers it, each with the provider's logo. Picking one runs the
  login in a terminal tab (Decision 6). With no such part installed, the
  dialog says so and offers Plugins mode. "Paste a key" is not offered yet.

### Needs

- Before: "Run login" and "Login expired" on profiles, through the agent
  part's `not-logged-in` and `login-refused`. "Key for <host> found in
  <agent>: no provider plugin [Add plugin…]". "The login was refused" with
  no button.
- After: login needs show wherever they are met (Decision 9): a profile
  with no login on the profile only, an expired login of a profile on the
  profile (agent part) and on its account (provider part), an OTTY own
  login on the account only as "Log in again". Acting in any place fixes
  all. The "no provider plugin" need goes,
  since no agent part reports keys any more. The plugin needs (update
  waits, part off, plugin failed, format changed) stay. Found by OTTY lists
  profiles and accounts in two separate groups with no "found in" links.

## WIT sketch

Only what changes. Comments are short. Exact names are for the issues.

```wit
// agents-agent/0.1.0/types.wit: only what both contracts share
interface types {
  use otty:host/types@0.1.0.{host-error, need};
  variant read-error { not-supported, host(host-error), unexpected(string),
                       needs-user(need), failed(string) }   // login-refused goes
  // identity and login-kind go. limit-window and credits move to the provider.
}
```

```wit
// agents-agent/0.1.0/agent.wit
interface agent {
  /// The agent's default config folder only (Decision 4).
  profiles: func(agent: string) -> list<profile-place>;
  /// Found, not logged in (with its need), or the folder is gone. No
  /// connection: whose login it is, is the provider part's business.
  detect: func(profile: profile) -> profile-status;
  version: func(profile: profile) -> option<string>;
  session-places: func(profile: profile) -> list<place>;
  read-sessions: func(profile: profile, item: place-item) -> result<list<session>, read-error>;
  spend-places: func(profile: profile) -> list<place>;
  /// `answer` keeps its provider and model for prices, and loses `login`.
  read-answers: func(profile: profile, item: place-item) -> result<answers, read-error>;
  launch: func(profile: profile, session: option<string>) -> command;
  // gone: logins, login-refused, records login, login-ref, connection, billing
}
```

```wit
// agents-provider/0.1.0/provider.wit
interface provider {
  use otty:host/types@0.1.0.{command, need};
  use otty:host/keys@0.1.0.{given-key};
  use otty:agents-agent/types@0.1.0.{read-error};

  /// A folder where a login of this provider may sit.
  variant login-folder {
    /// A profile of an agent part of the same plugin.
    profile(tuple<string, string>),      // agent part id, folder
    /// OTTY's own hidden profile for this part.
    own(tuple<string, string>),          // own login id, folder
  }

  enum account-kind { subscription, key, organization }

  record account {
    id: string,                          // stable within the plugin
    name: string,                        // "you@example.com", "key ••••a1f3"
    kind: account-kind,
    plan: option<string>,
    found-in: list<login-folder>,
    needs: list<need>,                   // such as "Login expired [Run codex login]"
  }

  /// The accounts in `folders`, and needs of folders with no login.
  record found { accounts: list<account>, needs: list<need> }

  accounts: func(provider: string, folders: list<login-folder>) -> found;
  /// Every way the part can: the server with the login, or the agent run
  /// in one of the account's folders.
  numbers: func(provider: string, account: account, given: list<given-key>)
          -> result<account-numbers, read-error>;
  read-bill: func(provider: string, account: account, given: list<given-key>,
                  days: day-range) -> result<bill, read-error>;
  /// The agent's official login command pointed at `folder`, such as
  /// `codex login` with CODEX_HOME=<folder>.
  log-in: func(provider: string, folder: string) -> command;

  // account-numbers, money, key-limit, extra-usage, day-range, bill stay.
  // limit-window and credits move here. gone: found-profile, login-facts,
  // account-facts, the key argument of numbers and read-bill, account().
}
```

The manifest of a provider part names the agent parts whose profiles it
reads, and whether it offers "Log in with":

```toml
[[agents.provider]]
id = "openai"
logins_of = ["codex"]       # agent parts of this plugin; also the "Log in with" label
log_in = true               # offers Add account → Log in with Codex
run = ["codex app-server", "codex login"]
http = ["chatgpt.com", "api.openai.com"]
# files it reads inside each login folder, keys as handles
login_files = ["auth.json"]
```

Permissions are applied by context as today. The part may read its
`login_files` and run its programs only inside the login folders OTTY
passes to the current call, so a profile the user added in any folder
works without a wider glob.

## What OTTY stores

`~/.config/otty/agents.json`, changed in place:

```json
{
  "default_profiles": { "codex": { "name": "Codex personal" } },
  "added_profiles": [
    { "agent": "codex", "folder": "~/.codex-work", "name": "Codex work" }
  ],
  "hidden_profiles": [ { "agent": "claude-code", "folder": "~/.claude-old" } ],
  "accounts": [
    {
      "plugin": "openai",
      "id": "chatgpt:acct_123",
      "name": "Personal",
      "hidden": false,
      "given_keys": [ { "slot": "admin", "file": "~/.secrets/openai-admin" } ]
    }
  ],
  "own_logins": [
    { "id": "6f0c1e2a", "plugin": "openai", "provider": "openai", "agent": "codex" }
  ],
  "found_report_shown": true,
  "keyring_allowed": false
}
```

- `accounts` keeps only the user's choices, keyed by plugin + account id:
  a name, Hide, and the keys given for `ask-key` needs with a slot. No
  provider reference, no key place, no fingerprint.
- `own_logins` lists the hidden profiles. The folder is always
  `<data dir>/accounts/<id>/` and is not stored. The login inside it is
  written by the agent program, never by OTTY.
- Removing a plugin keeps its entries, as today. Its hidden profile
  folders stay until the user removes the account, so reinstalling the
  plugin brings the account back.

## What it supersedes

Issues of spec `2026-11-19-agent-plugins`:

- **29** (a provider reads its accounts every way it can, duplicates are
  fine) and **31** (every plugin reads a host, each copy hides alone):
  replaced by Decisions 5 and 8. "Every way it can" stays, but only inside
  the plugin's own folders.
- **34**, the login parts (OpenCode gives the login it uses, accounts
  named by login): OpenCode has no provider part now.
- **43** (a stable `user-id` for nameless logins), never started: the
  plugin's stable account id replaces it.
- Parts of **20** (`login-refused` in the agent contract),
  **26** (a refused key of an added account), **38** (account chips),
  **40** (the By account switch).

Decisions of earlier specs:

- `2026-11-19-agents-accounts`: Decision 2 (finding extra folders and
  accounts in profile files, joining them), Decision 4 (tokens tied to
  accounts), the Used by, Pays through, By account and Account line parts
  of Decision 6, Add account by key place in Decision 7. Decision 3 (never
  renew another agent's login) stays and is kept by Decision 7 here.
- `2026-11-19-agent-plugins`: in Decision 4 the bullets "Agent part"
  (logins), "Provider part" (`login-facts`, `found-in`), "No fight over
  names", "Accounts and profiles are not plugins" (`merge_accounts`) and
  "Hidden accounts are left out of every total". The sections "Identity
  and links do not depend on a provider plugin" and the account rows of
  "What the UI shows".

Code that goes or is rewritten (not a full list):

- `crates/agents/src/`: `account.rs` (`merge_accounts`, identities),
  `plugin_agent.rs` (`account_of`), `refused_login.rs`,
  `account_money.rs` and `money_estimate.rs` (spend estimate of a key from
  tokens), `key_fingerprint.rs` where only joining uses it, the added
  accounts of `given_key.rs` and `saved_profiles/saved_accounts.rs`,
  the extra folders of `profile_list.rs`.
- `crates/app/features/agents/core/src/`: `model/used_by.rs`,
  `model/session_account.rs`, `model/account_chip.rs` and the Pays through
  of `model/profile_page.rs`, the account parts of `model/project.rs` and
  `model/usage_groups.rs`, `model/found_in.rs` (becomes Source),
  `add_account_dialog.rs` and `key_place_field.rs` (the key place part).
- Plugins: `plugins/opencode/src/logins.rs` and its login needs,
  `plugins/openai` and `plugins/anthropic`: the login listing moves from
  the agent part to the provider part.

## Open questions

- **macOS Keychain.** Claude Code on macOS keeps its login in the Keychain
  item `Claude Code-credentials`, not in its folder. Does
  `CLAUDE_CONFIG_DIR` give a separate Keychain entry, so a hidden profile
  does not log the user's own Claude Code out? CodexBar chose not to keep
  its own Claude login, maybe for this reason. Until checked on a Mac,
  "Log in with Claude Code" may be Linux only.
- **Money of a profile on a subscription.** Besides the "at API prices"
  mark, should the profile say it is on a subscription (it no longer knows
  its account), or show nothing more?
- Answered 2026-11-23: a cost the agent writes is shown as it is
  (Decision 3), the Claude login is `claude auth login --claudeai`, OTTY
  reads its own logins on the usual schedule while it runs and runs
  nothing in the background when it is closed, and whether a login
  expired is the plugin's call on both sides (Decision 9).

## Later

- **Paste a key.** Add account → a provider part that accepts keys → the
  user pastes the key, OTTY keeps it in the system keyring and gives the
  part a handle. This brings DeepSeek and OpenRouter accounts, and covers
  keys an agent keeps for another provider (a DeepSeek key in Claude
  Code's `settings.json`).
