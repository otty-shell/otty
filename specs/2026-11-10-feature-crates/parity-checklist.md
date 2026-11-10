# Parity check list: old app and new app

This list compares the new app (`crates/app/`) with the old one (`app/`)
before the old one is removed (issue 15). Do each step in both apps and tick
the box of each app where the result is as written. When a result differs,
write what you saw under the line and open an issue in `issues/`.

How to start each app:

- old app: `cargo run -p otty`
- new app: `cargo run -p otty-app`

Run only one app at a time. Both apps read and write the same files in
`~/.config/otty` (see "Start-up"), so a change saved in one app shows in the
other at its next start. **Copy `~/.config/otty` somewhere before you start
and put it back at the end.**

Shortcuts below are written for Linux. On macOS: Ctrl Shift K → Cmd K,
Ctrl Shift B → Cmd B, Ctrl Shift O → Cmd Shift E, Ctrl Shift F → Cmd Shift F,
Ctrl , → Cmd ,, Ctrl Shift T → Cmd T, Ctrl Shift W → Cmd W,
Super Shift D → Cmd D, Ctrl Shift D → Cmd Shift D, Ctrl Shift 1–5 → Cmd 1–5,
Ctrl Shift M → Cmd Shift M.

## Known differences — check they are acceptable

These differences were written down while the new app was built (issues
05–18). Look at each one and decide whether it may stay.

| # | Difference | From | Old | New |
|---|---|---|---|---|
| K1 | **New launch… under an open menu.** In the New session menu and on the start card, New launch… is now the app command `NewLaunch`. If a menu is open and a press reaches the tile, the new app closes the menus and opens the launch editor; the old app dropped the press. Every menu draws a dismiss layer, so the press may never reach the tile; check with each menu open. See [issue 20](issues/20-new-launch-under-an-open-menu.md). | 10, 12 | [ ] | [ ] |
| K2 | **No focus outside Terminal mode.** When a menu or dialog closes in Containers mode, no terminal pane gets the keyboard. The old app focused a pane that was not on screen. This is the spec rule ("Overlays"). | 11 | [ ] | [ ] |
| K3 | **Start-up toasts appear a moment earlier.** A broken settings, launches or pinned modes file is told while the app is built, not after the first update, so its five seconds start a moment before the first frame. | 06 | [ ] | [ ] |
| K4 | **Shell files after a Settings save.** Saving a new shell in Settings writes the shell integration files in the background. The old app wrote them inside the update. In both apps the new shell reaches new tabs one message later. | 09 | [ ] | [ ] |
| K5 | **`HOME` that is not valid Unicode.** The new app finds `~/.config/otty` with the raw `HOME` value; the old app read `HOME` as text and fell back to the system temp folder when it was not valid Unicode. Only such a `HOME` behaves differently. | 18 | [ ] | [ ] |
| K6 | **Palette after "Switch to Terminal mode".** The palette gives the active pane its keys back for this entry, and Terminal mode also focuses the pane on a mode change. The result should be the same: the pane has the keys. | 08, 11 | [ ] | [ ] |
| K7 | **A picked command that can no longer run** closes the palette and nothing takes the keyboard. Same as the old app; listed so the owner confirms it is wanted. | 08 | [ ] | [ ] |
| K8 | **Resize grips under the palette.** The window edges can still be dragged while the palette is open; every other menu turns them off. Same as the old app. | 08 | [ ] | [ ] |

## Start-up

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| S1 | Start with your usual `~/.config/otty`. | The window opens with your saved theme and language, your pinned modes in the mode switcher and your launches in the Launches section. | [ ] | [ ] |
| S2 | Look at the first tab. | One terminal tab opens on its own, runs your saved shell (or `$SHELL`), and is listed under your computer's host name in Sessions. | [ ] | [ ] |
| S3 | In that terminal run `ls -la ~/.config/otty`. | With zsh: `otty.zsh` and `.zshrc` are there, freshly written. With bash: `otty.bash` and `otty.bashrc`. The prompt shows command blocks (shell integration works). | [ ] | [ ] |
| S4 | Quit. Move `~/.config/otty` away so it does not exist, then start. | Default settings (system language, OTTY Dark, `$SHELL`), Containers pinned, no launches, no toast. The folder is created with the shell files only. | [ ] | [ ] |
| S5 | Quit. Write `{` into `settings.json`, start. | Defaults are used and a toast says "The settings file is broken, so the defaults are used: …". The bell counts 1. | [ ] | [ ] |
| S6 | Quit. Write `{` into `quick_launches.json`, start. | No launches and a toast "Could not read the launches: …". | [ ] | [ ] |
| S7 | Quit. Write `{` into `modes.json`, start. | Containers pinned and a toast "Could not read the pinned modes: …". | [ ] | [ ] |
| S8 | Quit. Break all three files at once, start. | Three toasts. Open the bell list: newest first, so the order from the bottom is settings, launches, pinned modes. | [ ] | [ ] |
| S9 | Quit. Make `settings.json` a folder (`mkdir`), start. | Defaults and a toast "Could not read the settings: …". | [ ] | [ ] |
| S10 | Put the saved files back. In each app change the theme, pin or unpin a mode and rename a launch, then quit and start the other app. | The other app shows the same theme, pins and launch: both apps read and write `settings.json`, `modes.json` and `quick_launches.json` in `~/.config/otty`. | [ ] | [ ] |

## Window frame

### Header

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| H1 | Look at the header. | Mode switcher, the title of the active tab, the search field, panel buttons (left panel, files, focus mode), bell, gear, window buttons. | [ ] | [ ] |
| H2 | Drag the header; double-click it; press Minimize, Maximize, Close. | The window moves; maximizes and restores; minimizes; closes. | [ ] | [ ] |
| H3 | Press the search field in the header. | The palette opens with its input focused. | [ ] | [ ] |
| H4 | Press the left panel, files and focus mode buttons. | Each toggles its panel; focus mode hides both panels. | [ ] | [ ] |
| H5 | Press the gear; press the bell. | Settings opens; the notification list opens under the bell. | [ ] | [ ] |

### Panels, focus mode and resize

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| P1 | Drag the edge between the left panel and the main area, then the edge of the right panel. | Both panels resize; terminals reflow to the new size. | [ ] | [ ] |
| P2 | Drag a panel edge until the panel closes, then reopen it with its button. | It comes back at the width it had before. | [ ] | [ ] |
| P3 | Ctrl Shift F. | Focus mode: both panels hide and a note "Panels are hidden … to show them" fades in. Ctrl Shift F again brings them back. | [ ] | [ ] |
| P4 | In focus mode try to drag where the panel edge was. | Nothing drags. | [ ] | [ ] |
| P5 | Drag each window edge and corner (Linux). | The window resizes; it does not get smaller than 800 × 600. | [ ] | [ ] |
| P6 | Make the window small, then large again. | Panels keep their share; nothing is cut off; the start card can still be scrolled and each tile pressed. | [ ] | [ ] |

## Modes

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| M1 | Press each mode in the switcher; use Ctrl Shift 1 and Ctrl Shift 2. | Terminal and Containers switch; the left panel, main area, right panel and status bar change with the mode. | [ ] | [ ] |
| M2 | Ctrl Shift 3–5 with only one pinned mode. | Nothing happens. | [ ] | [ ] |
| M3 | Ctrl Shift M, or the "All modes" button. | The All modes menu opens: Pinned with "n of 4", Other modes. Esc closes it. | [ ] | [ ] |
| M4 | Unpin Containers, then pin it again. | The switcher changes at once; after a restart the pins are kept. | [ ] | [ ] |
| M5 | Make `modes.json` read-only (`chmod 444`), then pin or unpin. | A toast "Could not save the pinned modes: …". | [ ] | [ ] |
| M6 | Switch to Containers and back to Terminal. | The tabs and the active tab are kept; the active pane has the keys again. | [ ] | [ ] |
| M7 | In Containers mode, press the sessions button in the status bar. | Terminal mode opens. | [ ] | [ ] |

## Terminal

### Tabs and sessions

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| T1 | Ctrl Shift T three times. | Three tabs, each listed in Sessions under this computer; the list scrolls to the new tab; the status bar counts the sessions. | [ ] | [ ] |
| T2 | Press a tab in Sessions. | It comes to the front, its pane has the keys, the header title and the Files panel follow it. | [ ] | [ ] |
| T3 | Ctrl Shift W. | The active tab closes; the next one comes to the front with focus. With the last tab closed, the start card shows and nothing has focus. | [ ] | [ ] |
| T4 | Ctrl Shift W in Containers mode or with no tab. | Nothing happens. | [ ] | [ ] |
| T5 | `cd /tmp` in a terminal. | The tab title and the status bar folder change; the Files panel shows `/tmp`. | [ ] | [ ] |
| T6 | Run `exit`. | The tab closes (or the pane, in a split). | [ ] | [ ] |
| T7 | Press the "New session" header menu (chevron of Sessions). | Menu at the pointer: New terminal, Split current pane right, your top-level launches, New launch…. Each item runs and closes the menu. Esc closes it. | [ ] | [ ] |
| T8 | Close all tabs and use the start card. | "Start a session" with New terminal and New launch… tiles; each works. | [ ] | [ ] |

### Splits and focus

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| F1 | Super Shift D, then Ctrl Shift D. | The focused pane splits right, then down; the new pane has the keys; the status bar counts the panes. | [ ] | [ ] |
| F2 | With "Equalize panes" on, then off (Settings), split again. | On: all panes get the same size. Off: the focused pane is halved. | [ ] | [ ] |
| F3 | Split until there is no room. | A toast "No room for another pane…". | [ ] | [ ] |
| F4 | Click into another pane; drag a split edge. | Focus moves; panes resize. | [ ] | [ ] |
| F5 | Each pane header shows the machine and folder. | This computer's name; folders under home start with `~`. | [ ] | [ ] |
| F6 | In a focused terminal press every app shortcut once. | Each runs exactly once (one new tab, one split, one palette …), and the terminal does not get the key. | [ ] | [ ] |
| F7 | Plain Ctrl D in a shell. | EOF reaches the shell (not a split). | [ ] | [ ] |

### Pane menu

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| PM1 | Right-click in a pane. | Menu at the pointer: Copy selection, Paste, Copy content / prompt / command (on a block), Split right, Split down, Close pane. | [ ] | [ ] |
| PM2 | Use each item. | Each does its action and closes the menu; after closing, the pane has the keys. | [ ] | [ ] |
| PM3 | Open the menu and type letters. | Nothing reaches the shell. Esc closes the menu. | [ ] | [ ] |
| PM4 | Open the menu, then left-click outside it, then middle-click outside it. | The menu closes; the click does nothing else. | [ ] | [ ] |

## Quick launch

### Launches section and editor

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| Q1 | Launches chevron menu: New launch, New folder. | New launch opens a "Create Quick Launch" editor tab; New folder starts naming a folder in the tree. | [ ] | [ ] |
| Q2 | Fill a Custom launch (program, arguments, workdir, env, After exit) and Save. | It shows in the tree and in the New session menu; `quick_launches.json` is written. | [ ] | [ ] |
| Q3 | Save with an empty title, a missing program, a bad port. | The editor shows the error for each field. | [ ] | [ ] |
| Q4 | Press a Custom launch. | A tab opens running it; "running" shows on the row while it starts. | [ ] | [ ] |
| Q5 | Launch with a missing program / missing workdir. | A toast "Could not launch "…": Program not found…" / "Working directory not found…". | [ ] | [ ] |
| Q6 | Try each After exit: Close, Close on success, Keep open. | The tab closes, closes only on exit code 0, or stays with "Process exited with code …". | [ ] | [ ] |
| Q7 | Switch the language in Settings, then open the launch editor. | The "optional" marker of the fields is in that language. | [ ] | [ ] |
| Q8 | Make `quick_launches.json` read-only and change the tree. | One toast "Could not save the launches: …" (not one per change). | [ ] | [ ] |

### SSH launches

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| SSH1 | Create an SSH launch (host, port, user) to a server that asks for a password. Press it. | A dialog "Password for user@host" with the launch name; the field has the keys. | [ ] | [ ] |
| SSH2 | Type a wrong password, press Connect. | The dialog stays: "The server rejected the password. Try again." | [ ] | [ ] |
| SSH3 | Type the right password. | A tab opens, listed under the SSH host in Sessions; Terminal mode shows if you were in Containers mode. | [ ] | [ ] |
| SSH4 | SSH launch with an identity file that has a passphrase. | A dialog "Passphrase for <path>"; a wrong one says "Wrong passphrase. Try again."; the right one opens the tab. | [ ] | [ ] |
| SSH5 | Identity file that does not exist. | A toast "Identity file not found: …". | [ ] | [ ] |
| SSH6 | Esc or Cancel in the dialog. | The dialog closes, no tab; the active pane has the keys back. | [ ] | [ ] |
| SSH7 | Unreachable host. | A toast "SSH connection failed: …". | [ ] | [ ] |

### Context menu and inline edit

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| QC1 | Right-click a launch, a folder, and empty space in the tree. | Menus with Edit, Rename, Duplicate, Remove / Delete, Create folder, Create quick launch, Kill (for a running launch). | [ ] | [ ] |
| QC2 | Use each item. | Edit opens "Edit: <title>"; Duplicate adds a copy; Remove/Delete removes; Kill stops the run. | [ ] | [ ] |
| QC3 | Rename: type a name, press Enter. | The name changes; the active pane has the keys again. | [ ] | [ ] |
| QC4 | Rename: a name a sibling already has. | "A sibling with this title already exists." | [ ] | [ ] |
| QC5 | Rename, then Esc. | The edit is cancelled. | [ ] | [ ] |
| QC6 | Rename, then press anywhere else (a tab, a Files row, a header button) or a shortcut. | The edit is cancelled and the press runs. Hovering a Files row or the title bar keeps the edit. | [ ] | [ ] |
| QC7 | Rename, then type in the terminal behind. | Typing goes to the name field, not the shell. | [ ] | [ ] |
| QC8 | Drag tree rows to reorder or move into a folder. | They move and the file is saved. | [ ] | [ ] |
| QC9 | Drag the left panel edge while a context menu is open. | The menu closes. | [ ] | [ ] |

## Explorer (Files panel)

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| E1 | Ctrl Shift O. | The Files panel toggles. | [ ] | [ ] |
| E2 | Look at it with a local tab in front, with no tab, and with an SSH tab in front. | A local tab: its folder, folders first. No tab yet: "No active folder". An SSH tab: note what shows; both apps should show the same. | [ ] | [ ] |
| E3 | Open and close folders; hover rows. | Folders open and close; hover highlights. | [ ] | [ ] |
| E4 | Create or delete a file in the shown folder from the terminal. | The list updates on its own. | [ ] | [ ] |
| E5 | Press a file. | It opens in a new tab with the saved Default editor (Settings), not an unsaved one. | [ ] | [ ] |
| E6 | Open a folder you may not read (`chmod 000`). | A toast "Could not open the folder "…": …". | [ ] | [ ] |

## Palette

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| PA1 | Ctrl Shift K. | The palette opens with focus in its input. Sessions grouped by machine, launches in tree order, then Actions with their shortcut labels. | [ ] | [ ] |
| PA2 | Type to filter; Up/Down; Enter. | The list filters; the active row moves; Enter runs it and closes the palette. | [ ] | [ ] |
| PA3 | Pick a session; a launch; a panel toggle; "Switch to Terminal mode". | It runs; afterwards the active pane has the keys. | [ ] | [ ] |
| PA4 | Pick Settings or a Containers mode entry. | It runs; no pane takes the keys. | [ ] | [ ] |
| PA5 | Ctrl Shift K while the palette is open. | It stays open with the typed text and the input focused. | [ ] | [ ] |
| PA6 | Esc; click outside. | The palette closes; the active pane has the keys. | [ ] | [ ] |
| PA7 | Open it in Containers mode. | Split and Close session are not listed. | [ ] | [ ] |
| PA8 | Press a header panel button while the palette is open. | The first press only closes the palette. | [ ] | [ ] |

## Settings

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| ST1 | Ctrl , (or the gear). | The dialog opens over a dimmed window, at its top. | [ ] | [ ] |
| ST2 | Change language, shell, editor, Equalize panes, theme; press Save. | Saved and applied at once; the dialog stays open and clean. New tabs use the new shell. | [ ] | [ ] |
| ST3 | Change something, then Reset. | The fields go back to the saved values. | [ ] | [ ] |
| ST4 | Change something, then Esc / Cancel / click on the dimmed area. | The dialog closes and the edits are dropped. In Terminal mode the active pane has the keys. | [ ] | [ ] |
| ST5 | Ctrl , or the gear while Settings is open. | Nothing: the edits stay. | [ ] | [ ] |
| ST6 | While open: shortcuts, the bell, the header search, Sessions presses, keys. | All ignored. Window buttons, the title bar drag and the window edges still work. | [ ] | [ ] |
| ST7 | Make `settings.json` read-only and Save. | A toast "Could not save the settings: …". | [ ] | [ ] |
| ST8 | Open Settings in Containers mode. | It opens over Containers mode; the mode stays. | [ ] | [ ] |

## Notifications

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| N1 | Cause a toast (for example a launch with a missing program). | A toast for about five seconds, with a cross; the bell badge counts it. | [ ] | [ ] |
| N2 | Press the bell. | The list opens: newest first, relative times ("just now", "n min ago"); the badge clears. | [ ] | [ ] |
| N3 | Press a row cross; "Clear all". | Only that row goes, the list stays open; Clear all empties it ("No notifications"). | [ ] | [ ] |
| N4 | Esc; click outside. | The list closes; the toasts stay. | [ ] | [ ] |
| N5 | A toast while Settings is open. | The toast lies under the dimmed area and its cross can not be pressed. | [ ] | [ ] |

## Containers

### Engines and lists

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| C1 | Containers mode with no engine installed / Docker stopped / Podman socket off / no access. | The matching card with its commands to copy and "Check again"; the status bar says the same in short. It checks again on its own every few seconds. | [ ] | [ ] |
| C2 | Start the engine while the card shows. | The lists appear on their own. | [ ] | [ ] |
| C3 | With Docker and Podman both running. | Both engines in the tree; each opens and closes. | [ ] | [ ] |
| C4 | Open Containers, Images, Volumes, Networks, and a Compose project. | Each table loads; empty lists say "No containers" etc.; the status bar shows counts. | [ ] | [ ] |
| C5 | Start or stop a container from a terminal (`docker run`, `docker stop`). | The table updates on its own. | [ ] | [ ] |
| C6 | Stop the engine while a list shows, then start it again. | The engine shows as unreachable, then comes back and reloads. | [ ] | [ ] |
| C7 | Press column headers. | The table sorts by that column, again to reverse. | [ ] | [ ] |
| C8 | Type in the search: a name, then `key:value` and Enter. | Rows filter; tags appear and can be removed one by one or with Clear all; "Nothing matches this search." when empty. | [ ] | [ ] |
| C9 | Press Refresh. | The list reloads. | [ ] | [ ] |
| C10 | Press a port link and a Compose link in a row. | The port opens in the browser; the Compose link opens its project. A browser that cannot start gives a toast "Could not open the link: …". | [ ] | [ ] |
| C11 | Scroll a long table, drag a panel edge across it, drag the table's scroll bar near a panel edge. | Dragging the panel edge never scrolls the table; the bar only scrolls. | [ ] | [ ] |

### Inspector and logs

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| CI1 | Press a row. | The right panel opens (if hidden) with the inspector on Overview. | [ ] | [ ] |
| CI2 | Overview links (image, volume, network, port, Compose). | Each opens its item in its list, clearing the search tags. | [ ] | [ ] |
| CI3 | Logs tab of a running container. | The last 500 lines, new lines appear; Follow sticks to the end; "n new lines" when scrolled up. | [ ] | [ ] |
| CI4 | Restart the container while on Logs. | The logs start again for the new run. | [ ] | [ ] |
| CI5 | Hide the right panel, then show it, while on Logs. | The stream stops while hidden and starts fresh when shown. | [ ] | [ ] |
| CI6 | Inspect tab: Show empty, Raw JSON, Copy JSON. | The tree, empty fields shown or hidden, raw JSON, JSON in the clipboard. | [ ] | [ ] |

### Run form and create dialogs

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| CR1 | Containers list → Run container. | The Run form opens in the main area; the right panel shows what the image sets on its own. | [ ] | [ ] |
| CR2 | Type an image that is here, then one that is not. | "On this computer, size" / "Not on this computer yet …" depending on the pull switch. | [ ] | [ ] |
| CR3 | Add ports, variables, volumes (a folder and `~/x`), network, restart, auto-remove. | Errors for a bad port, a path not starting with `/`, a missing folder; restart is disabled with auto-remove. On SELinux the `:z` switch shows. | [ ] | [ ] |
| CR4 | Press Run. | The container starts and shows selected in the list; the form closes. A failure gives a toast and keeps the form as typed. | [ ] | [ ] |
| CR5 | Run with a missing image and the pull switch on; cancel during the pull. | The pull shows "n of m"; cancelling stops it. | [ ] | [ ] |
| CR6 | Images → Pull image; Volumes → Create volume; Networks → Create network. | The dialog opens with focus in its first field; Pull shows progress; Create adds the item. Esc or Cancel closes it; a press on the dimmed area does nothing. | [ ] | [ ] |

### Actions and row menu

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| CA1 | Press "⋯" on a row; right-click a row. | The row menu opens at the row / pointer with that resource's actions. | [ ] | [ ] |
| CA2 | Start, Stop, Restart, Pause, Unpause a container. | The action runs and the list reloads. A failure gives a toast "Could not … : …". | [ ] | [ ] |
| CA3 | Shell on a running container. | A new terminal tab "shell <name>" runs a shell in it; Terminal mode shows. | [ ] | [ ] |
| CA4 | Remove a container, an image (with and without Force), a tag, a volume, a network. | A confirm dialog with the right text and switches ("Also remove named volumes", "Force …"); Remove runs it. Built-in networks and used volumes say why they can not go. | [ ] | [ ] |
| CA5 | Prune stopped / dangling / unused on each list. | A confirm dialog with the count; Prune runs and the list reloads. | [ ] | [ ] |
| CA6 | Pull again on an image; Run on an image. | Pull again opens the pull dialog; Run opens the Run form with that image. | [ ] | [ ] |
| CA7 | Open the row menu, then type a search that hides its row. | The row menu closes. | [ ] | [ ] |

## Overlays and Esc

The layer order, bottom to top: content and toasts; mode overlays (pane menu,
Launches context menu; Containers row menu and dialogs); header menus and the
All modes menu; the palette; the notification list; Settings; the SSH secret
prompt.

### Esc for each overlay

| # | Open, then press Esc | Expect | Old | New |
|---|---|---|---|---|
| O1 | Palette | Closes; pane has the keys (Terminal mode). | [ ] | [ ] |
| O2 | Notification list | Closes; toasts stay. | [ ] | [ ] |
| O3 | All modes menu | Closes. | [ ] | [ ] |
| O4 | New session / Launches header menu | Closes. | [ ] | [ ] |
| O5 | Pane menu | Closes. | [ ] | [ ] |
| O6 | Launches context menu | Closes. | [ ] | [ ] |
| O7 | Inline edit (Rename, New folder) | The edit is cancelled. | [ ] | [ ] |
| O8 | Settings | Closes and drops unsaved edits. | [ ] | [ ] |
| O9 | SSH secret prompt (Esc with focus in the field too) | Cancels the launch. | [ ] | [ ] |
| O10 | Containers row menu | Closes. | [ ] | [ ] |
| O11 | Containers confirm dialog | Cancel. | [ ] | [ ] |
| O12 | Containers pull/create dialog (Esc with focus in a field too) | Cancel. | [ ] | [ ] |

### Two overlays at once

For each pair, open the first overlay, make the second come up, then press
Esc twice.

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| OO1 | Start an SSH launch to a slow server that asks for a password, and open the palette before the prompt comes. | The prompt lies over the palette and takes the typing. Esc 1 cancels the prompt; Esc 2 closes the palette. | [ ] | [ ] |
| OO2 | The same with the notification list open. | Prompt over the list; Esc 1 prompt, Esc 2 list. | [ ] | [ ] |
| OO3 | The same with the All modes menu open. | Prompt over the menu; Esc 1 prompt, Esc 2 menu. | [ ] | [ ] |
| OO4 | The same with the New session header menu open. | Prompt over the header menu; Esc 1 prompt, Esc 2 menu. | [ ] | [ ] |
| OO5 | The same with the pane menu open. | Prompt over the pane menu; typed text never reaches the shell. | [ ] | [ ] |
| OO6 | The same with the Launches context menu open, and with a Rename in progress. | Prompt on top; Esc 1 prompt, Esc 2 menu / edit. | [ ] | [ ] |
| OO7 | The same with Settings open. | Prompt over Settings; a press on the prompt's dimmed area does not close Settings. Esc 1 prompt, Esc 2 Settings. | [ ] | [ ] |
| OO8 | The same in Containers mode with the row menu open, then with a confirm dialog open. | Prompt over them; Esc 1 prompt, Esc 2 the Containers overlay. | [ ] | [ ] |
| OO9 | Under the prompt, press a shortcut (Ctrl Shift T, Ctrl Shift K). | Nothing happens; the typed secret stays in the prompt. | [ ] | [ ] |
| OO10 | In Containers mode open a confirm dialog while an SSH launch from Terminal mode is still connecting; let it open its tab. | Terminal mode shows; the confirm dialog is hidden, not closed. Back in Containers mode it is there again. | [ ] | [ ] |
| OO11 | With a Containers dialog open, press Ctrl Shift 1. | Nothing: a dialog ignores shortcuts. | [ ] | [ ] |
| OO12 | With a menu open, press a shortcut (for example the palette open and Ctrl ,). | The menu closes and the shortcut runs. | [ ] | [ ] |
| OO13 | With a menu open, press a Sessions tab. | The menu closes and the tab comes to the front. | [ ] | [ ] |
| OO14 | Toasts with every overlay. | Toasts always lie under menus and dialogs. | [ ] | [ ] |
| OO15 | Resize grips with each overlay open. | Off under every menu and dialog except the palette (K8). | [ ] | [ ] |

## Theme and language

| # | Do | Expect | Old | New |
|---|---|---|---|---|
| L1 | Settings → Theme OTTY Light, Save. | The window and all open terminals switch at once; new terminals too. | [ ] | [ ] |
| L2 | Back to OTTY Dark. | Same. | [ ] | [ ] |
| L3 | Settings → Language: each language, Save. | All texts switch at once, including the open dialog, menus and the status bar; Chinese, Japanese and Korean letters use the shapes of that language. | [ ] | [ ] |
| L4 | Language: System. | The system language is used. | [ ] | [ ] |
| L5 | Restart after each change. | The saved theme and language are there from the first frame. | [ ] | [ ] |
