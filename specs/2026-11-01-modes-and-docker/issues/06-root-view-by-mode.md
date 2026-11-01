# 06 — Root view by mode: switcher and menu in the left panel

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01, 04
Spec: [../spec.md](../spec.md)

## What

The root view asks the active mode what goes in each slot (spec Decision 5).

- Left panel: kit `ModeSwitcher` with the mode name at the top, then the mode's content. Terminal mode keeps Sessions and Launches exactly as today.
- All modes menu: kit `ModeMenu` as an overlay anchored under "⋯". Esc and a click outside close it, like the header menus.
- Main area, right panel and status bar switch with the mode. Docker mode shows a temporary placeholder until tickets 09 and 10.
- Panel open/closed and widths stay shared (sidebar widget). When the left panel is hidden, the switcher is hidden too.
- Terminal tabs keep running in other modes. Coming back to Terminal mode syncs the grid size (`SyncTerminalGridSizes`) and focuses the active pane.
- This ticket changes `app/src/view.rs`; say so on the commit.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: clicking a tab changes the mode; clicking "⋯" opens the menu; Esc closes it.
- Tests: switching to Docker and back keeps the tabs and the active tab; grid size sync is emitted on return.
- The regression checklist of `specs/2026-10-31-workspace-layout` passes in Terminal mode.
