# 01 Agents and providers as data: the registry

Status: ready-for-agent
Blocked by: none

## Why

Spec Decisions 1 and 15, "Links in the interface". Today `Provider` is a
closed enum (`crates/agents/src/provider.rs`) and 26 files under
`crates/app/` match on it; `AgentId` wraps `&'static str` and
`AgentDescriptor` is a set of constants; `otty-app-theme` picks icons by
name (`icons::agent_icon`, `icons::provider_icon`). With plugins, agents
and providers are data. The UI must switch to that data first, while
today's in-core readers still run, so nothing breaks during the
migration and every later issue only swaps where the data comes from.

## What to build

1. In `otty-agents`: `ProviderId` and `AgentId` as owned strings;
   `ProviderEntry` (id, name, icon bytes, usage page(s), hosts, Add
   account kind `key` / `admin-key` / none, the reader: built in, or the
   plugin (id, version, repository) and the part id); `AgentEntry` (id,
   name, icon bytes, program, the reader). One `Registry` with lookups by
   id and by host (a host matches when it equals a provider host or ends
   with `.` and one; today's `PROVIDER_DOMAINS` rule). It holds one entry
   per agent id, provider id and provider host (spec Decision 4: which
   plugin serves an id is the user's choice, issue 08).
2. The in-core agents and providers register themselves as "built in"
   entries with today's ids, names, hosts, usage pages and the icons of
   `assets/svg/`. `Provider` and `AgentDescriptor` constants go away;
   what used their flags (`has_bill`, `has_stats`, `writes_cost`,
   `addable`, `is_added_as_organization`) reads the entry or the reader.
3. Account identity: (provider reference, id / organization id / email /
   key fingerprint), where the provider reference is a `ProviderId` or a
   normalized host. A host reference resolves to an id when an entry
   claims the host; `merge_accounts` joins across both.
4. The app gets an `Arc<Registry>` with the agents feature's context. All
   26 files under `crates/app/` that use `Provider::` read names, icons,
   usage pages and hosts through it. `icons::agent_icon` and
   `icons::provider_icon` are removed. With no entry, the UI draws a
   letter icon (first letter of the name or host on a rounded square, in
   the kit's style) and shows the id or host as the name.
5. The cases of the spec table "What the UI shows" that do not need the
   plugin engine: provider not in the registry (account listed by host,
   letter icon, Used by and tokens and "~ money" kept, no Left/Limits
   rows, need "Key for <host> found in <agent>: no provider plugin [Add
   plugin…]" with the button disabled until issue 09); agent not in the
   registry (its profiles leave the lists; its `agents.json` entries
   kept).
6. `agents.json`: ids stay as saved today; `host:<host>` accepted as a
   provider for accounts of a provider no entry serves; an unknown id is
   kept and not read (as today).

## Done when

- Tests first: registry lookups by id and host; a key for a host with no
  entry gives an account by host and the need; the same key found in two
  agents is one account with or without an entry for its host; removing
  a provider entry keeps Pays through and Used by; removing an agent
  entry keeps its saved names and hidden items, and they come back when
  the entry returns; old `agents.json` files read the same.
- `grep -rn "Provider::" crates/app` finds nothing; no icon is chosen by
  an agent or provider name.
- The app looks and behaves as before with all built-in entries present.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/provider.rs`, `agent.rs`, `account.rs`,
`keyring_entry.rs`, `account_numbers/mod.rs`, `saved_profiles/`;
`crates/app/features/agents/core/src/` (model, view, reducer, needs,
menus, add account dialog), `crates/app/features/agents/io/src/`,
`crates/app/theme/src/icons.rs`, `crates/app/otty/src/dispatch/agents.rs`,
`guards.rs`; i18n for the new need.

## Owner checks by hand

The app shows the same accounts, profiles, chips and icons as before.
Hide the DeepSeek entry in a debug build (or a test build): the DeepSeek
key shows with a letter icon and the "no provider plugin" need.

## Comments
