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
