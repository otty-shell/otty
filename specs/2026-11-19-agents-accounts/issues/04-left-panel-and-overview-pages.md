# 04 Left panel, All accounts and All profiles

Status: ready-for-agent
Blocked by: 02, 03

## Why

The user sees accounts and profiles side by side (spec Decisions 1, 6).

## What to build

1. Left panel: Accounts (All accounts first, then accounts), Profiles (All
   profiles first, then profiles), Projects **unchanged** (its search and
   New session "+" stay as they are). Row numbers carry their word (spec
   Decision 1). Accounts "+" opens Add account once issue 09 is in; until
   then it is not shown.
2. The `Need` model: title, line, one action; collected per account and per
   profile; never mixed.
3. All accounts page: table Account, Provider, Now (row opens the account),
   Needs you with account needs only, as a plain to-do list.
4. All profiles page: table Profile, Agent, Version, Today, and Needs you
   with profile needs only.
5. Icons: All accounts (two cards, kit style) and All profiles (from the
   Agents mode icon, robots, no people) as new `assets/svg` files.

## Done when

- Tests first: the needs split; row texts with their words; the tables'
  view models; Projects section untouched (existing tests pass).
- The checks of `AGENTS.md` pass.

## Files

`crates/app/features/agents/core/src/view/left_panel.rs`,
`projects_section.rs` (untouched), new overview views; `assets/svg`; i18n.

## Owner checks by hand

Click All accounts and All profiles; the needs lists hold only their kind;
project search and "+" work as before.

## Comments

Built (2026-11-19):

- Left panel: Accounts (All accounts with its count, then one row per
  account: provider logo, "Provider · login", its number), Profiles (All
  profiles with its count, then the profiles), Projects unchanged. The
  Accounts header has Refresh only; its "+" comes with issue 09. The
  sections share the panel height as before, now three of them.
- Row numbers with their word (`model/short_number.rs`): an account shows
  "42 % used" (most used window, waiting color from 80 %), "3 760 left",
  "$44.87 this month" from the bill, or "~ $42.10 this month" estimated
  with `AccountMoney` over the tokens of its profiles; a profile shows
  "1.8M today", "no sessions", or its need ("needs login"). "—" (or no
  number in the panel) while nothing is known.
- `Need` model (`model/needs.rs`): place, title, line, one action;
  `account_needs` and `profile_needs` are collected apart. The Found by
  OTTY report and both pages use them; the to-do list view is shared
  (`view/needs.rs`). `NeedAction` moved to `need.rs`; on a page its button
  sends `FixNeed` (the report keeps `FoundReportNeed`, which closes it
  first).
- Pages (`model/overview.rs`, `view/overview.rs`): All accounts (Account,
  Provider, Now) and All profiles (Profile, Agent, Version, Today), a
  row or its name opens the account or the profile, then "Needs you".
  A need state in Now or Today is a warn badge.
- `Profiles::accounts` (one row per account) and `first_with`.
- Icons `assets/svg/all-accounts.svg` and `all-profiles.svg`;
  `icons::provider_icon` gives Anthropic and OpenAI the logos of their
  agents, other providers the All accounts icon.
- i18n: 8 new keys in the 10 languages; `tpl_agents_most_used` ("42 %")
  is removed, the existing `tpl_agents_percent_used` ("42 % used") is used.

Decided differently:

- Until the account page (issue 05), pressing an account shows the
  dashboard of the first profile it was found in, with the account row
  marked.
- Profile rows now show tokens today, not the limits or the money of the
  profile: limits and money are the account's (spec Decisions 1 and 5).
- The pages have Refresh but no "⋯" yet: Show hidden is issue 08, and
  opening Found by OTTY again from it is left to issue 08 too.
- The mode still opens on the first profile (as before), not on All
  accounts as the prototype does.
