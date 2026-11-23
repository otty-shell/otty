# 05 A provider part finds its own accounts, with stable ids

Status: done
Blocked by: 01, 03, 04

## Why

Decision 5: the provider part owns accounts, found from the logins of
its own plugin's profiles, and never sees the profiles of another
plugin. Decision 8: for each account the part tells a stable id, a
display name, a kind, where it was found, its numbers and its needs.
Whether two logins are one account is decided inside the plugin, and
OTTY joins nothing across plugins (Decision 1). Decision 11 says what
each official plugin does, Decision 12 that the contracts change in
place.

Today an agent part lists the logins of a profile, OTTY joins the logins
of all plugins into accounts (`merge_accounts`) and lends a provider
part a key handle per account. That made copies (issues 29 and 31) and
nameless accounts (OpenCode's ChatGPT login).

This is the biggest slice: the contract, the host, OTTY's account model
and every official plugin change together, since the plugins must build
against the new contract. The agent contract keeps its login functions
until issue 06, OTTY just stops calling them here.

## What to build

1. **The provider contract**
   (`crates/plugins/sdk/wit/agents-provider/0.1.0/provider.wit`), as the
   spec's WIT sketch:
   - `login-folder` with one case for now, `profile(tuple<string,
     string>)` (agent part id, folder). The `own` case comes with issue
     08.
   - `account-kind` (`subscription`, `key`, `organization`), the record
     `account` (`id` stable within the plugin, `name`, `kind`, `plan`,
     `found-in`, `needs`) and `found` (`accounts`, `needs`).
   - `accounts(provider, folders) -> found`,
     `numbers(provider, account, given)`,
     `read-bill(provider, account, given, days)`.
   - Gone: `found-profile`, `login-facts`, `account-facts`, the `key`
     argument of `numbers` and `read-bill`, and `account()`.
   - `limit-window` and `credits` move here from
     `agents-agent/0.1.0/types.wit`.
   - `login-refused` of `read-error` (`types.wit`) goes: a provider part
     that is refused says so with a need of the account ("Login expired"
     with its `run` command), in `account.needs` or as `needs-user` of a
     read.
   - The `needs` of `found` are the part's needs that belong to no
     account, such as a login file it does not understand. A profile
     with no login gives no need here: that need is the agent part's
     (Decision 9).
2. **The SDK.** `crates/plugins/sdk/src/agents/provider.rs`, the
   re-exports of `agents.rs` and the test host follow
   (`crates/plugins/sdk/src/test_host/`), so a plugin's tests can call
   `accounts` with folders and read their files and keys there.
3. **The host.** `ProviderCalls` (`crates/plugins/host/src/agents/provider.rs`)
   gets `accounts(folders)`, `numbers(account, given)` and
   `read_bill(account, given, days)`. The login folders of a call
   (issue 04) are the folders passed to `accounts` and the `found-in` of
   the account for the other two. The pace of a slow reading is kept by
   part and account id. The "Found logins" group of "<Part> can access"
   (`PermissionLine::GetsProviderKeys`) goes: a provider part gets no
   key that another part found.
4. **OTTY's account** (`crates/agents/src/`):
   - `account.rs` is rewritten: an `Account` is the plugin, the provider
     part, the part's id of the account, its display name, kind, plan,
     where it was found (agent part and folder) and its needs.
     `AccountIdentity`, `AccountLogin`, `merge_accounts`, `join_copies`,
     `is_same_account`, `is_same_login` and the provider reference of an
     account go.
   - `PluginProvider` (`plugin_provider.rs`) asks its part for accounts
     with the folders of the shown profiles of the agent parts the part
     names in its `logins` (issue 04), and only those. A part with no
     `logins` gets no folder and finds no account (DeepSeek and
     OpenRouter, Decision 11).
   - `read_account_numbers` and `read_bill` read an account through its
     own part only, with the keys given for its `ask-key` slots.
   - OTTY stops calling the agent part's `logins` and `login-refused`:
     `Agent::accounts`, `Agent::found_login`, `Agent::login_refused`,
     `plugin_agent.rs` `account_of`, `refused_login.rs`,
     `SourceError::LoginRefused` and the joining use of
     `key_fingerprint.rs` go.
   - `agents.json` (`saved_profiles/saved_accounts.rs`,
     `saved_profiles/file_content.rs`): an `accounts` entry is keyed by
     `plugin` + `id` and keeps only `name`, `hidden` and `given_keys`, as
     in "What OTTY stores". No reader of the old shape (Decision 12).
     Removing a plugin keeps its entries.
5. **OTTY's screen** (`crates/app/features/agents/core/src/`):
   - Accounts are kept by plugin + account id (`profiles.rs`,
     `state.rs`). The left panel shows one row per account of each
     provider part, logo + name as today: the user's name, else the
     part's display name (`model/account_name.rs`, with no "plugin when
     two plugins read it").
   - Hide and Rename act on plugin + account id (`hiding.rs`,
     `rename_dialog.rs`).
   - The account page's Account info is Provider, Name, Kind, Plan,
     Source and Read by. Source (`model/found_in.rs` becomes the source)
     is "Found in ~/.codex", one per folder. The account's needs show in
     its Needs from you as any need of a part. Issue 07 adds the rest of
     Decision 9.
   - The need "Key for <host> found in <agent>: no provider plugin
     [Add plugin…]" goes, with its texts.
   - Found by OTTY and the io crate
     (`crates/app/features/agents/io/src/account_numbers.rs`, `limits.rs`,
     `bill.rs`) follow the new account.
6. **The official plugins.** Bump each plugin's patch version and rebuild
   with `plugins/build.sh`.
   - **OpenAI** (`plugins/openai`): the provider part declares
     `logins` for `codex` (`CODEX_HOME`, keys of `auth.json`, and
     `sessions/**` and `archived_sessions/**` read only, which replace
     the `~/.codex*` globs of its permissions). `accounts` reads
     `auth.json` in each folder: a ChatGPT login is a subscription with
     a stable id from its account id (such as `chatgpt:<account id>`),
     named by its email (and workspace when the claims name one), with
     its plan. An API key is a key, its id and its name ("key ••••a1f3")
     from its fingerprint. One ChatGPT account in two folders is one
     account with two places. The code of `accounts.rs` and
     `auth_file.rs` moves from the agent side to the provider side.
     `numbers` reads every way it can, as today, now only in the
     account's folders.
   - **Anthropic** (`plugins/anthropic`): the provider part declares
     `logins` for `claude-code` (`CLAUDE_CONFIG_DIR`, keys of
     `.credentials.json`, `.claude.json` and `settings.json`). A default
     profile keeps its `.claude.json` in the home folder, outside its
     folder: that file stays a plain permission of the part
     (`~/.claude.json`). A Claude login is a subscription with a stable
     id from its account (`oauthAccount` of `.claude.json`), named by its
     email and organization. A Console key and an Anthropic key of
     `settings.json` are keys. A key of `settings.json` for another host
     (`ANTHROPIC_BASE_URL`, such as DeepSeek) is no account (spec
     scenario "Claude Code pointed at DeepSeek"). The code of `logins.rs`
     moves to the provider side. `numbers` uses the token, else the
     `/usage` probe with `CLAUDE_CONFIG_DIR` of one of the account's
     folders, as today.
   - **DeepSeek, OpenRouter**: `accounts` gives none (Decision 11). Their
     `numbers` code stays for "paste a key".
   - **Template**: the example provider finds its accounts in the
     example agent's profiles, with a stable id and a name.
   - **OpenCode** only rebuilds for the changed `read-error`.
7. Module docs follow. The public docs are issue 10.

## Done when

- Tests first. (Host, over the template): `accounts` with two folders
  gives the accounts found there, the part reads keys only inside them,
  `numbers` of an account reads in its folders. (OTTY): accounts come
  from each provider part with its plugin's profile folders only, two
  plugins never share an account, a name and Hide survive a new read by
  plugin + id, an account's need shows on its page, the account page
  shows Source "Found in <folder>" per folder, and no "no provider
  plugin" need shows. (Plugins): OpenAI gives one subscription for one
  ChatGPT account in `~/.codex` and in `~/.codex-work`, two for two
  accounts, a key with a name from its fingerprint, the same id after
  the tokens of `auth.json` are renewed. Anthropic gives a Claude login
  with its email, a Console key, and no account for a DeepSeek key of
  `settings.json`.
- Agents mode with Codex and Claude Code profiles shows the same
  accounts as before this issue, less the copies and the accounts of
  OpenCode's logins.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-23, done. A provider part finds its own accounts and OTTY joins
none.

Contract (changed in place): `provider.wit` has `login-folder` (only
`profile`, the `own` case comes with issue 08), `account-kind`,
`account` (`id`, `name`, `kind`, `plan`, `found-in`, `needs`), `found`,
and `accounts(provider, folders)`, `numbers(provider, account, given)`,
`read-bill(provider, account, given, days)`. `limit-window` and
`credits` moved here. Gone: `found-profile`, `login-facts`,
`account-facts`, `account()`, the lent key, and `login-refused` of
`read-error` (a refused or expired login is a need). The agent contract
keeps `logins` and `login-refused` until issue 06; OTTY no longer calls
them. The SDK trait follows (`accounts` has a default: none), and
`TestHost::as_provider_in(id, &[LoginFolder], ..)` gives the folders of
a call. The host's `ProviderCalls` has `accounts`, `numbers` and
`read_bill`; the folders of a call are the folders given to `accounts`
or the `found-in` of the account, and the pace of a slow reading is kept
by part and account id. "Found logins" (`GetsProviderKeys`) and the
"keys found for a host" lines are gone, with their four texts, so a new
host of a provider asks for nothing more. A provider part may now
declare `[[agents.provider.permissions.keys]]`, JSON fields of files only,
for a login file outside every profile folder (`~/.claude.json` of the
default `~/.claude`).

`otty-agents`: `Account` is the plugin, the provider part, the part's id
(`AccountKey`), a name, a kind (`Subscription`, `Key`, `Organization`),
a plan, `FoundIn::ProfileFolder { agent, folder }` and needs.
`AccountIdentity`, `AccountLogin`, `merge_accounts`, `join_copies`,
`is_same_login`, `read_by`, `can_be_saved`, `KeyFingerprint`,
`RefusedLogin`, `SourceError::LoginRefused`, `Agent::accounts`,
`found_login`, `login_refused`, `account_of` and the tokens by account
of the spend count are gone. `find_accounts` asks each provider part with
the shown profiles of the agent parts its `logins` name, of its own
plugin only; `read_account_numbers` and `read_bill` read through the part
that found the account. `agents.json` keeps an account as `plugin` + `id`
with `name`, `hidden` and `given_keys`; an entry of the old layout is not
read.

Screen: the accounts come with the profiles read (`LoadedProfiles.accounts`)
and are kept by plugin + id; the names are the user's name, else the
part's name ("OpenAI · you@example.com", no plugin after it); Hide and
Rename work for every account, keys too, so Rename… and Hide are never
off. Account info is Provider, Name, Kind, Plan, Source ("Found in
~/.codex", one line per folder) and Read by (seven new texts in ten
locales). An account's needs show on its page and in Needs you; the
needs of no account show with the needs of the accounts. The "no
provider plugin", "part is off" and "login was refused" needs of an
account are gone with their texts. Kept on purpose: the limits and the
bill are still read through the first profile an account was found in
(an internal schedule, nothing on screen), and the `AddPlugin` and
`TurnOnPart` actions of `NeedAction` stay unused for issue 07.

Plugins: `openai` 0.2.7 -> 0.2.8 (provider `logins` for `codex`:
`auth.json` keys, `config.toml`, `sessions/**`, `archived_sessions/**`;
ChatGPT `chatgpt:<account id>` named by email, key `key:<fingerprint>`
named `key ••••<last 4>`, an admin key an organization, a gateway login
none; an expired token is a `login-expired` need running `codex login`
with `CODEX_HOME`; the claims name no workspace, so the name is the
email alone). `anthropic` 0.2.7 -> 0.2.8 (`logins` for `claude-code`;
Claude login `claude:<accountUuid>` named "email · organization", a
prepaid Console login an organization `console:<uuid>`, keys
`key:<fingerprint>`, a key for another base URL none; a login is
expired only when `expiresAt` passed and no refresh token is left, since
Claude Code renews short tokens itself). `deepseek` and `openrouter`
0.1.2 -> 0.1.3 find no accounts; their reads take a key given under the
slot `key`, for "paste a key". `opencode` 0.1.6 -> 0.1.7 and the
template `example` 0.1.3 -> 0.1.4 (accounts from the key of `auth.json`
by fingerprint) rebuilt; `template.wasm` and the four test plugins of
the host were rebuilt.

Left for later issues: the agent contract's login functions and the
`login` of an answer (06), the needs wherever met and the Source line of
OTTY's own logins (07, 08), the public docs (10).
