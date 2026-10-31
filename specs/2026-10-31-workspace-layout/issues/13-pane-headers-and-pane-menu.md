# 13 — Pane headers and the pane context menu

Status: ready-for-agent
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
