# Error Notifications

Created: 2026-11-09
Author: ilyashvy@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Build

## Summary

**Goal:** when an operation fails, the user learns why from a short message
at the top of the window, a *toast*, and can read it again later in a list
behind a bell in the header. Before this, most such failures were only
written to the log, and some showed a one-off notice or an error tab.

## Decisions

1. **Toasts.** A toast shows at the top center of the window, newest on top,
   at most 3 at a time. Each stays for 5 seconds or until its cross is
   pressed. Toasts are no overlay: Esc leaves them and they take no keyboard
   focus. A dialog's dimmed backdrop lies over them, so a toast cannot be
   closed while a dialog waits for an answer.
2. **The list.** A bell in the header, right before Settings, and a palette
   entry open the list. It is a menu, like the other header menus. It keeps
   the last 100 notifications in memory only: nothing is saved, and a restart
   starts with an empty list.
3. **Rows.** Each row shows the error icon, the text and a relative age:
   "just now", "N min ago", "N h ago". A row lights up under the pointer. Its
   cross removes only that notification. The broom button clears the whole
   list. The list stays open after both.
4. **The badge** on the bell counts the notifications that came since the
   list was last opened: hidden at 0, "99+" from 100, never more than the
   list holds. Opening the list hides the toasts and marks everything seen.
   A new error while the list is open goes into it without a toast.
5. **Errors only.** Only failed operations become notifications: no success
   messages, no warnings, no hints.
6. **Inline form validation stays where it is**, next to its field.
7. **Repeating background failures stay log-only:** they are not started by
   the user and would flood the list. These are shell integration writes;
   the Files root load, its reloads and the watcher; folder entries that
   cannot be read.
8. **Removed:** the launch error tab, the notice above the Containers table
   and the notice on top of the Run form.

## Error sources

| Source | What the user sees now |
|---|---|
| Saving the pinned modes fails | Toast; the switcher keeps the choice |
| A launch from Launches fails before its tab opens | Toast naming the launch; no tab opens, the mode stays |
| A new tab's terminal does not start | The tab closes; toast naming the launch, or "Could not open a terminal" for a shell |
| A split's new terminal does not start | Toast; the pane is not split |
| A Containers action, pull or Run fails | Toast; the Run form stays open with every field as typed |
| Saving the launches fails | Toast for the first failure since the last good save; the save keeps trying every 200 ms |
| Saving the settings fails | Toast; the popup unlocks, the edits stay |
| Reading the settings fails when the popup opens | Toast; the popup keeps what it showed |
| The settings file is broken when the popup opens | Toast; the defaults are shown and applied |
| A folder pressed open in Files cannot be read | Toast naming the folder; the folder shows open and empty, as before |
| A link's opener (`xdg-open`, `open`) cannot be started | Toast |
| At start, the launches, settings or modes file cannot be read or is broken | Toast once the window is ready; the defaults are used, and the next edit writes over the file |
| At start, one of these files is missing | Nothing: this is the first start |

Stays as before:

- The Containers list-load error and the right panel errors show in place.
- Log only: the failures in Decision 7; an opener that starts but exits with
  an error later; unknown or extra names in `modes.json`, which are skipped.

## Acceptance Criteria

- [x] AC1: Toasts follow Decision 1 (notifications reducer and view tests).
- [x] AC2: The list, its rows, the broom and the badge follow Decisions 2–4
  (notifications widget, kit and `update.rs` tests).
- [x] AC3: Every source in the table above tells the user as described
  (reducer, storage and route tests).
- [x] AC4: Every notification template fills its `{error}` in all 10 locales
  (`i18n` tests).
- [x] AC5: `cargo +nightly fmt`, clippy with `-D warnings`, `cargo deny check`,
  `cargo test --workspace --all-features` and `cargo xtask arch` (0 new,
  0 stale) pass.

## Out of Scope

- Saving the list across restarts.
- Notifications for success, progress or warnings.
- Collapsing a folder in Files that failed to open.
