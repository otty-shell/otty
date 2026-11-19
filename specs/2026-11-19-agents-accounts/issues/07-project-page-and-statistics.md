# 07 The project page and the statistics popups

Status: ready-for-agent
Blocked by: 06

## Why

Spec Decision 6: projects show profiles and accounts; statistics stay
popups.

## What to build

1. Project page: Project info (Path, Profiles chips, Accounts chips,
   Sessions, First and Last activity), then the sessions table with a
   Profile column (agent icon, link). Keep every interaction of today:
   click, double click and Enter = Continue, right-click and the row "⋯"
   (Continue, Continue in a split, Copy resume command, Copy session ID,
   Show statistics), search, sort.
2. Project statistics popup (the button stays): facts plus the usage block
   with By profile | By model | By account. Money only from known prices;
   a line names the subscription profiles left out.
3. Session statistics popup: facts plus an Account line (written in the
   session; the profile's account; "one of 2 OpenAI accounts") and the
   usage block with By model | By account. No "By session" tab.
4. The Projects section of the left panel is unchanged.

## Done when

- Tests first: the Account line cases; the project sums by profile and by
  account; every session interaction still reaches its command (existing
  tests stay green).
- The checks of `AGENTS.md` pass.

## Files

Feature core `view/project.rs`, `stats_dialog.rs`, `sessions_table.rs`,
`menus.rs`; i18n.

## Owner checks by hand

Open a project, continue a session by double click, open both popups.
