# Regression Checklist: Workspace Layout

Manual flows from spec AC1. Walk them on a release-like build
(`cargo run -p otty --release`) before and after each app ticket. Later
tickets move buttons around (the tab strip becomes the Sessions list, the rail
goes away), so each step names the action, not the pixel: where a step says
"the tab", use the session row once ticket 10 lands; where it says "the
Explorer panel", use the Files panel once ticket 08 lands.

Record each walk in the ticket's `## Comments` as
`Checklist: <date>, <build/commit>, steps 1–N pass` and list any failing step
by number with what happened.

Setup before the first walk:

- A local shell with OTTY shell integration (zsh or bash).
- One SSH host you can reach whose private key has a passphrase (for step 9).
- Settings › Terminal › Default editor set to an editor that runs in a
  terminal, for example `nvim` or `nano`.
- Optional: back up `~/.config/otty/settings.json` and
  `~/.config/otty/quick_launches.json`; steps 10–15 and 19–21 change them.

## Terminals

1. Start OTTY.
   Expected: one terminal tab opens with your shell prompt and has keyboard
   focus; typing shows up in it.
2. Open a second terminal (today: "+" in the tab strip, or the add menu ›
   Create tab).
   Expected: a new tab opens, becomes active, and gets focus; the first tab
   keeps running.
3. Split the active pane with the shortcut (Super Shift D on Linux, Cmd D on
   macOS) and once more from the pane context menu (right-click › Split
   horizontally / Split vertically).
   Expected: each split adds a pane with its own shell; the new pane gets
   focus. With Settings › Terminal › Pane layout "Even out sibling pane
   widths" on, sibling panes get equal widths.
4. Click another pane in the same tab.
   Expected: focus moves to the clicked pane; typing goes to it.
5. Switch between the two tabs by clicking them.
   Expected: the clicked tab becomes active and its focused pane gets
   keyboard focus; the other tab's shells keep their state.
6. Close one pane (right-click › Close, or `exit` in it).
   Expected: only that pane closes; focus moves to a sibling pane.
7. Close the active tab with its close button, then close a background tab.
   Expected: closing the active tab activates the tab before it (the last tab
   if it was the first one); closing a background tab keeps the active tab.
   After the last tab closes, the main area is empty and nothing crashes.

## Launches

8. Run a command launch (click a custom launch, for example `htop`).
   Expected: a new tab opens with the command running (if setup takes more
   than a second, the launch row blinks until it does). While it starts,
   right-click › Kill cancels it. With "After exit: Close" the tab closes when the command
   exits; with "Keep open" the pane shows the exit code and closes on a key
   press.
9. Run an SSH launch whose key has a passphrase.
   Expected: the "Key passphrase required" dialog opens with focus in the
   input. A wrong passphrase shows "Wrong passphrase. Try again."; the right
   one opens a tab logged in to the host. Cancel closes the dialog and opens
   no tab.
10. Create a launch (add menu › Create quick launch), fill Title and Program,
    Save.
    Expected: the launch editor opens as a tab; after Save the tab closes and
    the launch appears in the Quick launch tree. An empty title or program
    shows an error and does not save.
11. Edit a launch (right-click › Edit), change a field, Save.
    Expected: the editor opens with the saved values; after Save the change
    is kept (run it to check).
12. Rename a launch and a folder (right-click › Rename, type, Enter).
    Expected: the row shows the new name; a name used by a sibling shows "A
    sibling with this title already exists." and is not applied.
13. Duplicate a launch twice (right-click › Duplicate).
    Expected: two copies appear with unique titles.
14. Delete a launch (right-click › Remove) and a folder (right-click ›
    Delete).
    Expected: the node is gone from the tree.
15. Drag a launch into a folder, then back to the empty area below the tree.
    Expected: the folder highlights as a drop target; the launch moves into
    it, then back to the root. Dropping a folder into its own child is
    refused.
16. Restart OTTY.
    Expected: the launch tree from steps 10–15 (names, order, folders, open
    or closed folders) is restored.

## Explorer

17. Open the Explorer panel, then `cd` to another folder in the active shell.
    Expected: the Explorer root follows the shell's folder after the prompt
    returns. Switching to another terminal tab moves the Explorer to that
    tab's folder; switching to Settings keeps the last folder.
18. Click a file in the Explorer.
    Expected: a new tab opens with the file in the default editor; quitting
    the editor closes the tab.

## Settings, theme and language

19. Open Settings (today: the gear in the rail), change Settings › Terminal ›
    Shell or Default editor, press Save, restart OTTY.
    Expected: Settings opens as a tab (a second click activates the same
    tab); the saved value is there after the restart.
20. Switch the theme between OTTY Dark and OTTY Light and press Save.
    Expected: the whole window and the open terminals change colors at once,
    without a restart.
21. Switch the language (for example to Deutsch) and press Save, then switch
    back to English.
    Expected: menus, the Settings tab title and the panels change language at
    once, without a restart.

## Window

22. Hide and show the side panel (today: click the logo in the title bar),
    collapse it with the rail toggle, and drag its edge wider, narrower and
    almost closed.
    Expected: the terminal area grows and shrinks to fit and the shell
    redraws at the new size; the panel cannot cover more than 80% of the
    window; dragging it almost closed collapses it.
23. Resize the window, toggle full screen and minimize from the title bar.
    Expected: terminals redraw at the new size; the window restores
    correctly.
