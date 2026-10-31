# 13 — Pane headers and the pane context menu

Status: resolved
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04
Spec: [../spec.md](../spec.md)

## What

- Each terminal pane gets a kit `PaneHeader` as its pane grid title bar (`widgets/terminal_workspace/view/pane_grid.rs`): program name, folder, machine `Chip`. The focused pane's header uses the focused variant. The terminal widget is not edited (spec Decision 4); check that grid size sync accounts for the header height.
- Data: program name and folder per pane come from what the terminal workspace already knows (shell name, `shell_cwd`); add them to `TerminalTabViewModel` as plain fields.
- Pane context menu (`pane_context_menu.rs`): same items and actions as today, grouped with separators as in `V4FileOpen`; English labels "Split right" / "Split down", shortcut labels from ticket 06.

## Done when

- View model tests for header data; regression checklist split/close/focus flows pass; terminal rows still fill the pane exactly (no clipped last line).

## Answer

Each pane now has a kit `PaneHeader` as its pane grid title bar (program, folder, machine `Chip`). The focused pane uses the focused variant. Pressing the header focuses the pane.

- `terminal_workspace/model.rs`: new `PaneHeaderViewModel` (program, folder, machine) with `new(program, cwd, home, machine)` (pure, tested) and `for_entry(entry, machine)`. Program is the terminal's current title (the shell name until the program sets a title); folder comes from the block metadata (`terminal_cwd_from_blocks`), with the home folder written as `~`. `TerminalTabViewModel` has a new `machine: &str` field.
- Machine chip: always `TargetKind::Local` with the label "Local" (new i18n key `pane_machine_local`, all 10 locales). Panes do not know if they are SSH yet. Ticket 10 only needs to fill `machine` (and the target kind) in `TerminalTabViewModel` / `view_pane_header` in `view/pane_grid.rs`.
- Pane menu (`view/pane_context_menu.rs`): same actions, grouped with separators as in V4FileOpen: copy selection, paste | block copies (only with a selected block) | Split right (shortcut from `AppShortcut::SplitPane.label()`), Split down | Close pane. i18n keys renamed `split_vertically` to `split_right`, `split_horizontally` to `split_down`; "Close" is now "Close pane". All 10 locales updated.
- Grid height: the terminal widget sizes itself from its layout box, so the 30px title bar needs no extra sync (no edit to `crates/ui/terminal`). The stored `pane_grid_size` is only used to keep the menu on screen; the menu height now counts separators (`pane_menu_height`, tested).

Tests (written first, run red with a `todo!()` body, then implemented): 5 header view model tests (home to `~`, home itself, outside home, same-prefix sibling, no folder); 2 menu height tests. Existing i18n tests check all locales have every key.

Not verified: the visual look and "no clipped last row" in the running app.

### Comments

Owner must walk regression checklist steps 2, 3, 4, 6 (split, focus by click, close pane via menu and `exit`), 8 (command pane header, exit banner), 21 (language switch: new menu strings), 23 (resize: last row not clipped). Also check by eye: header look in both themes, folder updates after `cd`, menu grouping.
