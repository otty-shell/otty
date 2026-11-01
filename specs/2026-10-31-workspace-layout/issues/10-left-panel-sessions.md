# 10 — Left panel: Sessions replaces the tab strip

Status: resolved
Type: task
Class: B
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 03, 07
Spec: [../spec.md](../spec.md)

## What

Open tabs become `SessionRow`s in the upper `PanelSection` "Sessions" (spec
Decision 2). `TabsState` and its reducer do not change.

- Enable the `hostname` feature of `nix` (spec Decision 10) and read the host name once at start; fall back to "This computer".
- View model: each tab gets a machine: SSH host for tabs started by an SSH launch, this computer otherwise. Find where a tab's launch is known (quick launch services / tab open effects) and pass the machine in the view model; if that needs a new field on `TabItem`, it is set only when the tab is opened. Groups keep first-open order; rows keep tab order.
- Row title = tab title; state word for Settings ("settings"), launch editor ("editor"), error tab ("failed"). Click activates (`ActivateTab`), middle click or the close button closes (`CloseTab`), any tab strip action that exists today (check `tab_bar.rs` before removing it) stays reachable from the row.
- Header action: New session (plus) opens a terminal tab; ticket 16 turns it into a split button.
- Remove `widgets/tabs/view/tab_bar.rs` and the tab strip from `view.rs`. Delete `TabStrip` from the kit if nothing else uses it, and its gallery section.

## Done when

- View model tests: grouping by machine, order, state words, active row.
- `iced_test`: row press activates, close closes.
- Tab flows of the regression checklist pass.

## Answer

Open tabs are now `SessionRow`s in the "Sessions" `PanelSection` at the top
of the left panel, grouped by machine. The top tab strip is gone.

### View model

`app/src/widgets/tabs/model.rs`, built by `TabsWidget::vm(host_name)`:

- `TabsViewModel { groups: Vec<SessionGroupViewModel>, session_count }`
- `SessionGroupViewModel { name, target: TargetKind, sessions }`: `name` is
  this computer's host name (or "This computer") or the SSH host.
- `SessionViewModel { tab_id, title, state: Option<SessionState>, is_active }`
- `SessionState::{Settings, Editor, Failed}` from `TabContent` (Settings,
  launch editor, launch error); a terminal has no state word.

Groups keep the order of their first open tab (tab ids grow with opening
order); rows keep tab order. Two tabs on the same SSH host share one group.

The view (`app/src/widgets/tabs/view/session_list.rs`, same props pattern as
the Launches section) draws a `SessionGroup` per machine with the note
"this computer" or "ssh", and a `SessionRow` per tab: press → `ActivateTab`,
close cross and middle click → `CloseTab`, state words "settings", "editor"
(muted) and "failed" (error color). The header has an accent "New session
(Ctrl Shift T)" plus button → `OpenTerminalTab` (the old strip's plus).
Closed, the header shows the session count. The rows sit in a scrollable
(`SESSION_LIST_SCROLL_ID`), so the existing `ScrollBarToEnd` effect now
scrolls the session list. The section open flag is `sessions_open` in the
sidebar state (`SidebarIntent::ToggleSessions`, `is_sessions_open`), like
`launches_open`. A 1px divider separates Sessions and Launches, as in the
design.

### How the machine is determined

- `TabItem` has a new `machine: TabMachine` (`ThisComputer` or
  `Ssh(host)`), set only when the tab opens. `TabsIntent::OpenCommandTab`
  carries it; every other open path uses `ThisComputer`.
- The quick launch router (`events/quick_launch.rs`) maps
  `OpenCommandTerminalTab` with `QuickLaunch::ssh_host()` (new): an SSH
  launch gives `Ssh(host)`, a command launch `ThisComputer`. Files opened in
  the editor from the Files panel run on this computer.
- The host name is read once at start (`services::read_host_name`, nix
  `gethostname`, feature `hostname` enabled in the workspace and `nix` added
  to the app for `cfg(unix)`), kept as `App::host_name: Option<String>`.
  `TabMachine::name(host_name)` falls back to the translated "This computer"
  when it is `None`, resolved at view time so a language switch updates it.
- Pane headers (ticket 13): `TerminalWorkspaceWidget::vm` now takes the
  machine name and `TargetKind` of the active tab; the chip shows
  e.g. "fedora" with a local dot or the SSH host with an SSH dot. The i18n
  key `pane_machine_local` ("Local") became `this_computer`.
- Files panel dot (ticket 08): stays `TargetKind::Local`, on purpose. The
  panel follows shell tabs only (`shell_cwd_for_active_tab`), and shell tabs
  always run on this computer; SSH launches open command tabs. Taking the
  active tab's machine would draw an SSH dot next to a local folder. The
  stale "until ticket 10" comment now says this.

The reducer's close order is unchanged and still follows tab ids: closing
the active tab activates the tab opened before it (the last tab if it was
the first one), even when that tab sits in another machine group.

### Removed

- `app/src/widgets/tabs/view/tab_bar.rs` and the strip in `view.rs`
  (`view_tab_area`); the main pane is now just the active tab's content.
- Kit `TabStrip` (`components/tab_strip.rs`, its `mod.rs` line, two gallery
  sections and the gallery's tab state); nothing else used it. The glyph
  `plus` is now test-only (only component tests use it). `size::TAB` stays
  in the kit for ticket 22, like `RAIL`.
- `layout::pane_grid_size` no longer subtracts the 34px strip height.
- `TabsViewModel.tabs`, `active_tab_id` and the unused `has_tabs`.

Kit addition: `SessionRow::on_middle_press(msg)`.

i18n (all 10 locales): `sessions_panel_title`, `new_session`,
`this_computer_note`, `ssh_machine_note`, `session_state_settings`,
`session_state_editor`, `session_state_failed`; `pane_machine_local`
renamed to `this_computer`.

### Tests (written first, run red with `todo!()` bodies, then implemented)

| Where | Tests | What they pin |
|---|---|---|
| `tabs/model.rs` | 7 | no tabs; grouping local and SSH; SSH group first when opened first; one group per host; "This computer" fallback; state words; only the active row is active |
| `tabs/types.rs` | 4 | machine name with host name, fallback, SSH host; target kind |
| `tabs/reducer.rs` | 2 | command tab keeps its SSH machine; shell, Settings, editor and error tabs run on this computer |
| `tabs/view/session_list.rs` (`iced_test`) | 7 | row press → `ActivateTab` (moved from the old `tab_bar` test); close cross → `CloseTab`; middle click → `CloseTab`; New session → `OpenTerminalTab` with the shell name; title → toggle; group names, notes and state word show; closed section hides rows and shows the count |
| `events/quick_launch.rs` | 2 | SSH launch opens a tab on its host; command launch on this computer |
| `quick_launch/types.rs` | 1 | `ssh_host` |
| `sidebar/reducer.rs` | 1 | Sessions section starts open and toggles without effects |
| `layout.rs` | 1 replaced | pane grid takes the full screen height (the tab strip tests are gone) |
| kit `session_row.rs` | 1 | middle press sends its message |

All of ticket 01's tab close-order tests pass unchanged.

## Comments

- **Checklist walk: not done** (the agent cannot run the GUI). Owner, please
  walk these steps of `regression-checklist.md` ("the tab" is now the
  session row in the left panel):
  - 1: start, one terminal row under the host-name group, active.
  - 2: second terminal from the Sessions "+" (and Ctrl Shift T).
  - 5: switch tabs by clicking rows; the active row is highlighted.
  - 7: close the active row with its cross (the tab before it becomes
    active, the last one if it was the first), close a background row, and
    middle-click a row; closing the last row leaves an empty main area.
  - 8, 9: a command launch shows under this computer, an SSH launch under
    its host with the "ssh" note; the pane header chip names the machine.
  - 10, 11: the launch editor row shows "editor"; a failed launch row
    "failed".
  - 17: Files still follows the active shell tab.
  - 19: Settings row shows "settings"; a second open activates the same row.
  - 21: switch language: Sessions title, notes, state words and the
    tooltip change.
  - 22: toggle Sessions and Launches; open many tabs and check the list
    scrolls and Launches keeps the rest of the height.
  - 23: resize: the terminal now fills the height the strip used.
- Also check by eye: the host name in the group label and pane chip, and
  that the last terminal row is not clipped now that the strip is gone.
- `ScrollBarToEnd` scrolls the session list to its end; with groups the
  newest tab is not always the last row (a new local tab sits above an SSH
  group). Left as is because the reducer was not to change.
- A launch error tab is grouped under this computer, also for an SSH launch
  that failed: `OpenErrorTab` does not carry the launch.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok (the
  three old `yanked` warnings remain). `nix` was already in the lock; only
  the `hostname` feature is new.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): base
  `957f800` 76.80% → this ticket 77.21%.
- Round 4 (2026-11-01, owner): the close cross on a session row now shows only while the row is hovered (kit `SessionRow`, built with `iced::widget::hover`); the room for it is always reserved, so the state word does not move.
