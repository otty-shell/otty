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
