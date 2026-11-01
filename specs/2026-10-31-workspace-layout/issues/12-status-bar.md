# 12 — Status bar

Status: resolved
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 02, 10
Spec: [../spec.md](../spec.md)

## What

Kit `StatusBar` at the bottom of the window, as in `V4Workspace` (spec
Decision 9: a view in `widgets/chrome/view/status_bar.rs`, no new widget).

- Left: machine of the active session (dot + name), shell name, mono folder of the focused pane. Right: "N panes" for the active terminal tab, "N sessions".
- With no tabs (empty state): machine on the left, "OTTY <version>" on the right.
- In focus mode the status bar stays.
- A small `StatusBarViewModel` built from the tabs, terminal workspace and explorer view models; pure function.

## Done when

- View model tests: terminal tab, settings tab, no tabs, folder change, pane count after split and close (spec AC8).

## Answer

The kit `StatusBar` now sits at the bottom of the window, below the main area (it stays in focus mode).

- Left: machine of the active session (`plain_target` dot and name), program of the focused pane (its terminal title) and its folder in mono (`~` for home). Right: "N panes" for the active terminal tab and "N sessions". With no tabs: the machine on the left and "OTTY <version>" on the right. A Settings or editor tab shows only the machine and the session count. No tabs falls back to this computer's host name.
- `widgets/chrome/model.rs`: `StatusBarInput` and `StatusBarViewModel::new` (pure). `widgets/chrome/view/status_bar.rs`: the view. `view.rs`: `view_status_bar` builds the input and the bar is added to the root column.
- `TerminalWorkspaceWidget` gets `focused_pane_header`, `focused_program` and `pane_count`.
- i18n (10 locales): `status_pane_one`, `tpl_status_panes`, `status_session_one`, `tpl_status_sessions`, with `i18n::status_panes` and `status_sessions`.
- Files panel line (ticket 08 follow-up): "follows <program>" now uses `followed_program(focused pane program, default shell name)` in `terminal_workspace/model.rs`. With no terminal tab it falls back to the default shell name.

Tests (written first, red with `todo!()`, then implemented): 5 status bar view model tests (terminal tab, settings tab, no tabs, folder change, pane count across split and close), 3 `followed_program` tests, 2 `iced_test` view tests (terminal tab items, empty state with version).

Not verified: the visual look; pane count after a real split is tested only as the number passed into the view model (the count itself is `pane_grid::State::len()`).

## Comments

Owner, regression-checklist steps to walk:
- 1: start; the bar shows the host name, shell, folder, "1 pane", "1 session".
- 2, 3, 4, 6: split a pane and close it (menu and `exit`); "N panes" follows; click panes and the program and folder follow the focused pane.
- `cd` in a pane: the folder in the bar and pane header changes.
- 7: close the last tab; the bar shows the machine and "OTTY <version>".
- 8, 9: an SSH launch tab shows the SSH host with the SSH dot.
- 17: Files line says "follows <program>" and changes with the focused pane (for example while `nvim` runs).
- 19: open Settings; the bar shows only machine and session count.
- 21: switch language; pane and session texts change.
- Focus mode (Ctrl Shift F): the status bar stays.
- Check by eye in both themes.
