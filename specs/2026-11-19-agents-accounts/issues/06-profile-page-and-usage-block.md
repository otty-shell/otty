# 06 The profile page and the usage block

Status: ready-for-agent
Blocked by: 04

## Why

Spec Decision 6: the profile page shows usage by project, model and account
with tokens or money, in the OTTY style.

## What to build

1. The usage block (shared by the profile page and the statistics popups of
   issue 07): title row with two dropdowns (Today / 7 days / 30 days;
   Tokens / Money), a chart in the `DayBars` look (hourly for Today; the
   picked or hovered row lights its part of each bar in the accent), tabs
   By project | By model | By account above the table (name, value,
   share). Disabled options with their reasons. Defaults By model, 7 days,
   Tokens. No overflow at a narrow main area.
2. The profile page: Needs from you, info line, the usage block, Pays
   through chips, Profile info (Agent, Version, Folder, Projects, Sessions),
   Where the numbers come from. Replaces today's dashboard of a profile.
3. Kit: if `DayBars` cannot draw the lit part, extend it in `otty-ui-kit`
   (builder style, as the kit's other components).

## Done when

- Tests first: grouping and sums per tab and period; Money disabled for a
  subscription, By account disabled when accounts are not told apart; the
  lit part of the bars; the kit change in the kit's tests.
- The checks of `AGENTS.md` pass.

## Files

Feature core views (`dashboard.rs` and blocks replaced), the kit's
`day_bars`; i18n.

## Owner checks by hand

Switch period, measure and tab; pick a row; narrow the window with the left
panel open.

## Comments
- 2026-11-19, issue 05: the account page draws its "Used by" chips in `view/account_page.rs` (`profile_chip`); move that to a shared view for the Pays through chips instead of drawing them again. `found_in_text`, `needs_of_account` and `Profiles::found_with` are there to reuse too.

Built (2026-11-19):

- `otty-agents`: each counted day now also keeps its tokens split by
  project and model (`DaySpend::parts`, `ProjectModelTokens`), and today
  is split by the hour (`Spend::today_by_hour`, `HourSpend`), in the
  user's time zone; one place, `spend_tally`, fills both for every agent.
- Usage block: choices in `usage.rs` (`UsagePeriod`, `UsageMeasure`,
  `UsageTab`, defaults 7 days, Tokens, By model; kept while OTTY runs,
  the picked row is forgotten on another tab or profile); the sums in
  `model/usage_groups.rs`; the block in `model/usage_block.rs` and
  `view/usage_block.rs`. Title "Usage" with "191 in 7 days" (or "Last
  good numbers, …" greyed after a failed count), two compact drop-downs
  that open menus at the pointer, bars (24 hours for Today, this hour in
  the accent), tabs, table Name | Tokens or Money | Share. Picking a row,
  or the pointer over it, lights its part of every bar.
- Money is the estimate at the list prices ("~ $3.40", a note under the
  table). It is off for a subscription ("Paid by a subscription: there is
  no price per request") and while the prices are not read; By account is
  off when the profile has several accounts of a provider and the agent
  does not write which one paid (`used_by::untold_accounts`, shared with
  the Used by chips). Tokens no account paid for are "no account".
- Profile page (`model/profile_page.rs`, `view/profile_page.rs`) replaces
  the dashboard: header with Refresh and "⋯", Needs from you
  (`needs_of_profile`, with Log in), an info line ("Codex has no sessions
  yet…", "… is installed, but has no sessions and no logins yet…"), the
  sessions read failure, the usage block, Pays through chips (provider
  icon, account name, its number; a press opens the account), Profile
  info (Agent, Version, Folder, Projects, Sessions) and the sources (money
  of a per-token profile, then tokens).
- Chips drawn once in `view/chips.rs` for Used by and Pays through.
- Kit `DayBars`: `current(index)` puts the accent on another bar than the
  last, and `bar_with_part(value, part, tip)` lights a part of a bar at
  its bottom (no bar is the current one then).
- Removed: the dashboard, the tokens and top blocks, `TopTab`, the money
  totals of a profile (the money and limits are the account's now), and
  11 i18n keys no longer used; 20 new keys in the 10 languages.

Decided differently:

- The reason of By account off is a muted line under the tabs whenever it
  is off: iced segments report no pointer over a disabled one.
- The Money reason is the heading under a line in the measure menu, as
  the prototype; the drop-downs are the kit `NoteSelect` field with the
  feature's own menu, since the kit `Select` has no disabled options.
- The pointer over a row is found by a `mouse_area` over the table (row
  height from the kit sizes), so the kit `DataTable` is unchanged.
- The money of the block is always the estimate: a bill can not be split
  by project, model or account.
