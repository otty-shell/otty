# 08 Menus, rename and hide

Status: ready-for-agent
Blocked by: 05, 06

## Why

Spec Decision 7: the user's own names and what they do not want to see.

## What to build

1. "⋯" on account and profile pages and a right click on their rows (left
   panel and the overview tables) open the same menu at the pointer;
   Escape, a click away or a pick closes it; the selection does not
   change.
2. Account: Rename…, Open usage page (the provider's page in the browser),
   Hide / Show again. Profile: Rename…, Open folder (in the Files panel),
   Hide / Show again. Overview "⋯": Show hidden … (N) / Hide hidden ….
3. Hidden items are not read and leave lists, chips and needs.
4. Names and hidden items are saved in `agents.json`.

## Done when

- Tests first: the menu entries per kind; hide stops reads; names and
  hidden items saved and read back; an `agents.json` without them still
  reads.
- The checks of `AGENTS.md` pass.

## Files

Feature core `menus.rs`, reducer, state; `crates/agents/src/saved_profiles.rs`; io save/load; i18n.

## Owner checks by hand

Rename both ChatGPT accounts; hide one; Show hidden; restart OTTY and see
the names kept.

## Comments
- 2026-11-19, coordinator: moved here from issues 03/04 — the "⋯" of All accounts also has "Show Found by OTTY" (opens the Found by OTTY window again); both overview pages get their "⋯" here (Show hidden accounts/profiles).
- 2026-11-19, issue 05: the account page has Refresh but no "⋯" yet; add it here with the account menu.
