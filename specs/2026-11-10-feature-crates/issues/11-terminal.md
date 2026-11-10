# 11 Terminal

Status: needs-triage
Blocked by: 10

## What to do

Move `app/src/widgets/terminal_workspace` into `otty-app-terminal`, following
"Moving a feature" in the spec. It adds the `MainArea` trait to
`otty-app-slots`.

The terminal feature owns the live terminal sessions (see the spec,
"Exceptions"); creating one starts its shell.

Link it to the features already moved:

- tabs: opening a tab opens its terminal; `Closed { tab_id, new_active_id }`
  reaches the terminal, and the terminal decides on focus and selection;
- explorer: the folder of the active local terminal reaches the explorer as
  `SyncRoot`;
- modes: a tab whose terminal started sends `TabShown`;
- overlays: when the last overlay closes in Terminal mode, the active pane gets
  the keys back.

Behaviour to keep (from the old app):

- `SplitRight` and `SplitDown` (Linux Ctrl+Shift+D, macOS Cmd+Shift+D); a split
  that does not fit is refused with a notification only;
- the smallest pane is 100 px, for dragging and for the check; after the window
  shrinks, panes squeeze but stay inside the main area;
- a focused terminal takes the keys it handles, so a shortcut runs once;
- the pane context menu and closing a pane.

## Done when

- Opening shells in tabs, splits, focus, closing panes and tabs work as in the
  old app.
- `otty-app-terminal` depends on no other feature.
- The steps and checks in "Moving a feature" are done.

## Comments
