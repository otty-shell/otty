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

## Comments

Built (2026-11-19):

- Reads per account: each profile now comes with the accounts found in
  its files (`LoadedProfile::accounts`). The limits are read through the
  first profile of an account that wants a read; another profile of the
  same account waits for that read and gets a copy of the answer (one
  "Updated" time). The same for the bill of an organization. The sources,
  their order and the states are unchanged and still read with the files
  of one profile. A profile with no account found reads alone, as before.
- `AccountMoney` (`otty-agents`): the bill of an account when one was
  read, else the estimate over the tokens of the profiles that pay
  through it; tokens of a profile with several accounts of one provider
  that does not write which one paid are not split (count for none). It
  is for the account page (issue 05): the profile page still shows the
  profile's own estimate, or the bill.
- No budget: gone from `Profile`, `agents.json` (an old
  `monthly_budget` is read and dropped on the next save), the Edit
  profile dialog, the money rows and the i18n keys.
- Admin key on the account: `agents.json` gets `accounts`, each an
  organization by provider and organization id with its admin key place.
  The Edit profile field stays, but edits the key of the organization the
  profile pays through, so all its profiles share it. An old key in a
  profile entry is moved to the organization found in that profile when
  the profiles are read; the file changes with the next save.
  `read_bill` takes the agent and the key place instead of a profile.

Decided differently:

- Only an organization (`AccountIdentity::OrganizationId`) takes an admin
  key, so nothing keyed by a key fingerprint is ever saved. A profile paid
  by a key alone (settings `ANTHROPIC_API_KEY`, no Console login) has no
  organization: it shows no admin key field, and its old admin key stays
  in its profile entry, unused, until an organization is found there (or
  issue 09 adds organizations by hand).
- The limits and the bill are still kept per profile in the feature, as
  a copy per profile of the account's read, so the screens stay as they
  are; issues 04 and 05 can move them to an account state.
