# 31 Every plugin that serves a host reads it; each copy hides alone

Status: done
Blocked by: 29

## Why

Owner, 2026-11-22, after issue 29. Two choices of issue 29 go against
"duplicates are fine":

- A host that two plugins serve under different provider ids becomes the
  provider id of the first plugin (`Registry::provider_of_host`,
  `resolve`), so the second plugin never reads the account. Example: an
  account whose key goes to `api.example.com`; plugin A serves it as
  provider `cloud`, plugin B as provider `cloud-fork`. Today only A reads
  it; the owner wants two rows, one per plugin, as for two parts with one
  provider id.
- An account the user added by hand (a key given in OTTY) is one saved
  entry for every plugin that reads it, so hiding one copy hides them all.
  The owner wants each copy hidden on its own, as found accounts are.

## What to build

1. An account known by a host is read by every provider part that serves
   that host, whatever its provider id: one account row per plugin, each
   with its plugin's label, as in issue 29 point 4. Joining by identity
   stays inside one provider id; copies under different provider ids are
   never joined.
2. An added account keeps its one key (the user gives it once), but its
   name and hidden flag are saved per plugin copy, like a found account's
   (`"plugin"` in `agents.json`). Hiding one copy leaves the others shown;
   a hidden copy is out of every total and is not read (issue 29 point 6).
3. Spec (Decisions on providers and hosts, Decision 7) and public docs
   follow where they say which plugin reads a host or what hiding does.

No backward compatibility: Agents mode was never released.

## Done when

- Tests first: one host served by two plugins under two provider ids →
  two accounts, both read; an added account read by two plugins → hiding
  one copy keeps the other shown and counted; the saved file round-trips
  the per-copy flags.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-22, issue 31, done.

1. `merge_accounts` no longer turns a host into the provider id of the
   first plugin: an account read by no plugin yet gets one copy for each
   provider part of `Registry::readers_of` (every part with its id, or
   every part that serves its host, whatever its id), with the part's
   id as its provider and the part's plugin as `read_by`. Joining by
   identity then happens per provider id and plugin, so the same key
   found for the id `cloud` joins only the copy of `cloud`.
   `Registry::resolve` and `SavedProfiles::with_providers_resolved` are
   gone (they picked the first plugin). `Registry::is_same_provider` now
   says a host is the provider of every id whose part serves it, while
   two ids stay two providers, so the tokens a profile sent to a host
   are paid by each copy. `readers_of` lowers the case of a host as
   `provider_of_host` does; `provider_of_host` stays, the first part,
   only to name and draw a host. The plugin label of an account name now
   shows when a part of another plugin serves the same provider id or a
   host of the reading part (`Registry::is_shared`), not only the same
   id.
2. An added account keeps one entry with its key and no `plugin` (the
   key the user gave once); the name, the hidden flag and the given keys
   of each copy a plugin reads are an entry with the same `key` and that
   `plugin`, like a found account's. `is_of` matches the plugin for
   every account; `added_accounts` lists only the entries without a
   plugin; Edit key moves the entry and every copy, Remove forgets them
   all. Hiding one copy leaves the others shown, read and counted; a
   hidden copy is not read and is out of every total (the issue 29 code
   already kept hidden accounts out, by copy).
3. Spec (Decision 4 "No fight over names", "Accounts and profiles are
   not plugins", "Hidden accounts are left out of every total", the
   registry, "Identity and links do not depend on a provider plugin",
   the UI table; Decision 7 of 2026-11-19-agents-accounts) and the
   public docs (Plugins, Overview, Write a Plugin, What OTTY Reads and
   Sends) say so. No WIT or plugin change.

Tests: one host served by two plugins under two provider ids is two
accounts, both read (agents `account.rs`, app `reads.rs`, with their
labels); each copy pays for the host's tokens; `is_same_provider` and
`is_shared` for such a host; an added account read by two plugins,
hide one copy through the menu, the other stays listed, among the shown
accounts and read (app `hiding.rs`); `agents.json` round-trips the
per-copy name, hidden flag and given key, Edit key and Remove follow
every copy (`saved_accounts.rs`).

Left open for the owner:

- A saved entry kept by host (`host:api.example.com`) is the choice of
  the account no plugin reads only; once parts serve the host, each copy
  starts with no choice (before, it became the choice of the first
  part's id, which issue 29 had already stopped matching).
- A host's name, icon and price name (for tokens the agent sent to a
  host) still come from the first part that serves it.
- The label shows whenever a part of another plugin serves the same id
  or a host, even if that part reads no copy of this account (a key found
  for the id only).
- The given keys (slots such as `admin`) of an added account are per
  copy too, as for a found account.

- 2026-11-23, superseded by `specs/2026-11-23-accounts-apart-from-profiles/spec.md` (Decisions 1,
  5, 8 and 10). No host is read by every plugin that serves it: a provider
  part reads only the logins of its own plugin's profiles and its own
  logins, OTTY joins no accounts, and an account is kept by plugin +
  account id. Accounts added by hand, whose copies hid alone, are gone. An
  account two plugins find is still two accounts, each hidden alone.
