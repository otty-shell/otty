# Settings as a Popup over Every Mode

Created: 2026-11-09
Author: ilyashvy@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Build

## Summary

**Goal:** Settings stops being a tab. It opens as a popup window over the
active mode, Terminal or Containers, and does not switch the mode.

**The owner's rule (2026-11-09):**

- Tabs are only what the terminal widget runs: shells, command tabs and the
  quick launch editor.
- A mode is a different global task.
- Anything else, such as Settings, is a popup over every mode, not a tab.

This replaces Decision 2 of `specs/2026-10-31-workspace-layout/spec.md` for
Settings: Settings is no longer a row in the Sessions list.

### Before (research, 2026-11-09)

| What | Where |
|---|---|
| The gear, Ctrl , and the palette entry run `AppCommand::OpenSettings`, which opened the single Settings tab | `app/src/router/command.rs` |
| Opening or activating the Settings tab switched Containers mode to Terminal mode | `app/src/router/tabs.rs` (`shows_tab`) |
| The tab showed the form in the main area; the Sessions row said "settings" | `app/src/screens/terminal.rs`, `widgets/tabs/model.rs` (`SessionState::Settings`) |
| Settings was no overlay: shortcuts, the palette and the window kept working while it showed, and nothing had the keyboard | `app/src/guards.rs` |
| Edits stayed in the draft when the user left the tab; Equalize panes and the Files editor read that unsaved draft | `router/terminal_workspace.rs`, `router/explorer.rs` |

## Decisions

1. **A dialog.** Settings is `Overlay::Settings`, one of the dialogs
   (`Overlay::is_dialog`). While it is open, shortcuts, app buttons and clicks
   outside it do nothing; the window buttons still work. It takes the keyboard
   from every widget when it opens (`focus::nothing()`), so typed keys never
   reach a shell under it.
2. **Opening.** The gear, Ctrl , (Cmd , on macOS) and the palette entry open
   it in either mode, without a mode switch and without a tab. Opening it
   reads the settings file again, as opening the tab did.
3. **Already open (Q3).** The gear, the shortcut and the palette entry do
   nothing while it is open: the dialog rule ignores them, and opening an open
   popup changes nothing, so the edits stay.
4. **Closing (Q1).** Esc, the close cross in the title, and Esc inside the
   Shell or Default editor field close it. Closing drops the edits that were
   not saved: the draft goes back to the saved settings. After it closes, the
   active pane gets the keyboard back in Terminal mode
   (`SettingsEffect::Closed` → `TerminalWorkspaceIntent::FocusActive`); in
   Containers mode no pane is on screen, so nothing takes it.
5. **Save (Q2)** writes and applies the settings as before. The popup stays
   open and clean. Reset stays.
6. **Where it is drawn.** In the main area, between the title bar and the
   status bar. The layers, bottom to top: window content, mode overlays, the
   All modes menu, the palette, Settings, the SSH prompt, then the resize
   grips, which are empty while Settings is open. In `top_overlay` Settings
   comes right after the SSH prompt.
7. **Look.** The kit `Dialog` gets `width` (`size::DIALOG_WIDE`, 640 px) and
   `on_close` (a close cross in the title). The popup has the Appearance,
   General and Terminal sections, then Reset and Save. In a window too small
   for it, it keeps its width and scrolls both ways, like every dialog.
8. **Saved settings, not the draft (Q4).** Equalize panes and the editor that
   Files opens use the saved settings, never edits that are not saved yet.

## Acceptance Criteria

- [x] AC1: The gear, Ctrl , and the palette entry open Settings in Terminal and
  in Containers mode; the mode stays and no tab opens (`update.rs`,
  `router/command.rs` tests).
- [x] AC2: While Settings is open, shortcuts, app buttons and outside clicks do
  nothing; the window buttons work; typed keys do not reach a shell
  (`guards.rs` tables, `view.rs` tests).
- [x] AC3: Esc, the cross and Esc in the Shell or Editor field close it and drop
  unsaved edits; in Terminal mode the active pane gets its keys back (reducer,
  dialog view and `update.rs` tests).
- [x] AC4: Save keeps the popup open and clean; opening it again while it is
  open keeps the edits (reducer and `update.rs` tests).
- [x] AC5: The SSH prompt is drawn over Settings and takes its answers; Settings
  is drawn over Containers; it opens again at its top (`view.rs` tests).
- [x] AC6: There is no Settings tab, no "settings" Sessions row and no
  `SessionStateSettings` key.
- [ ] AC7: Equalize panes and the Files editor read the saved settings (Q4).
- [x] AC8: `cargo +nightly fmt`, clippy with `-D warnings`, `cargo deny check`,
  `cargo test --workspace --all-features` and `cargo xtask arch` (0 new,
  0 stale) pass.

## Out of Scope

- Settings that apply without Save.
- Launch error tabs: they move to the notifications plan.
- A confirmation before closing with unsaved edits: the owner chose to drop
  them (Q1).

## Manual checks

The regression checklists cover this:

- `specs/2026-10-31-workspace-layout/regression-checklist.md`, steps 24, 24b,
  25, 26, 26c and 27;
- `specs/2026-11-01-modes-and-docker/regression-checklist.md`, steps 18 and
  24a.
