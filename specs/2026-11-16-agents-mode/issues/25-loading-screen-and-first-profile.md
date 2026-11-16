# 25 The loading screen and the first profile

Status: ready-for-agent
Blocked by: 24

## Why

Spec Decision 10: Agents mode should not show an empty screen while it
reads, and should open on something useful.

## What to do

1. Until the first reading of the profiles is done, the main area shows the
   kit `Spinner` with "Looking for agents…", built the way Containers mode
   shows its spinner while it looks for engines (`crates/app/features/
   containers/core/src/view/main_area.rs`).
2. When it is done and nothing is selected, the first AI profile of the
   list is selected and its dashboard opens. A later refresh never changes
   the user's selection. With no profile at all, the existing "no agent was
   found" card shows.

## Done when

- Tests, written first, cover: the spinner before the first reading; the
  first profile selected after it; a selection the user made kept across a
  refresh; the card with no profile.
- The checks of `AGENTS.md` pass.

## Comments
