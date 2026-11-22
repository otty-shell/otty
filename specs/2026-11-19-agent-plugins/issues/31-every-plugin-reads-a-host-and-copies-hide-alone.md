# 31 Every plugin that serves a host reads it; each copy hides alone

Status: open
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
