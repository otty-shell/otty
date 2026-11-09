# Regression Checklist: Workspace Layout

Manual flows for the new window (spec AC1, AC4 to AC8). Walk them on a
release-like build (`cargo run -p otty --release`), once in OTTY Dark and once
in OTTY Light. Steps marked "both themes" need a look in each theme; the rest
only need one pass.

Window parts used below: the title bar (left panel toggle, search box, Files
toggle, focus toggle, Settings gear, window buttons), the left panel
(Sessions over Launches), the main area, the right panel (Files, closed by
default) and the status bar. Shortcuts on Linux; on macOS use Cmd (see the
table in the spec, Decision 3).

Record each walk in the ticket's `## Comments` as
`Checklist: <date>, <commit>, <theme>, steps 1-N pass` and list any failing
step by number with what happened.

Setup before the first walk:

- A local shell with OTTY shell integration (zsh or bash).
- One SSH host you can reach whose private key has a passphrase (step 12),
  and one with password login (optional, step 12).
- Settings > Default editor set to an editor that runs in a terminal, for
  example `nvim` or `nano`.
- Optional: back up `~/.config/otty/settings.json` and
  `~/.config/otty/quick_launches.json`; steps 13-19 and 24-26 change them.

## Window and panels

1. Start OTTY.
   Expected: no logo in the title bar; the left panel toggle is at its left
   end. The left panel shows Sessions with one row under your host name
   ("this computer") and Launches. The right panel is closed. The status bar
   shows host name, shell, folder, "1 pane", "1 session". The terminal has
   keyboard focus; typing shows up in it. Both themes.
2. Click the left panel toggle, then press Ctrl Shift B.
   Expected: the left panel hides and shows each time; the terminal resizes
   and the shell redraws. The toggle uses the accent color while the panel is
   open. Both themes.
3. Click the Files toggle, then press Ctrl Shift O.
   Expected: the right panel opens and closes. It shows "Files", the folder
   of the shell and "follows <program>".
4. Drag the edge of each panel wider and narrower; drag the right panel edge
   from the window's right side to open it (the handle sits next to the
   window resize grip).
   Expected: the panel follows the drag; both panels together never leave the
   main area less than 20%; dragging almost closed collapses the panel;
   dragging the right handle opens the panel and does not resize the window.
   Then drag a panel wider, close it with its toggle and open it again, by the
   toggle, the shortcut and the palette.
   Expected: it comes back at the same width each time, and so does the other
   panel. Close it again, then run "New folder" from the palette (left panel)
   or press a Docker row (right panel): also the same width.
5. Resize the window, toggle full screen and minimize from the title bar;
   double click empty title bar space; drag the window by empty title bar
   space.
   Expected: terminals redraw at the new size; the window restores correctly;
   double click toggles full screen; dragging moves the window.
6. Press Ctrl Shift F (or the focus toggle).
   Expected: both panels hide, the title bar stays, the status bar stays, and
   the "Focus mode" note shows at the top center with the shortcut. Click,
   select, scroll and right-click the terminal beside the note: all work; the
   note itself does nothing. Ctrl Shift F again restores the panels as they
   were. Ctrl Shift B or Ctrl Shift O in focus mode leave it. Both themes.

## Sessions and terminals

7. Open a second terminal with the Sessions "+" and with Ctrl Shift T. Hover
   the "+" and the chevron.
   Expected: each opens a new row, becomes active and gets focus; the first
   session keeps running. "+" and chevron show two tooltips.
8. Open the chevron menu.
   Expected: the menu opens at the cursor with a shadow; it lists New
   terminal, Split current pane right, New launch and your launches; a click
   outside or a right click closes it. The chevron keeps its frame while the
   menu is open. "Split current pane right" splits the focused pane to the
   right and focuses the new pane; with a launch editor row active it is dimmed.
   Both themes.
9. Split the active pane right with Super Shift D (Cmd D on macOS), down with
   Ctrl Shift D (Cmd Shift D on macOS), and with the pane menu (right-click >
   Split right / Split down); click another pane. Press plain Ctrl D in a
   shell.
   Expected: each split adds a pane with its own shell and focus. Each pane
   has a header with program, folder and a machine chip; the focused header
   is lit. The status bar says "N panes" and its program and folder follow
   the focused pane. With Settings > "Even out sibling pane widths" on,
   sibling panes get equal widths. The pane menu has a shadow in both themes
   and shows the shortcut next to Split right and Split down. Ctrl Shift D
   splits down and sends nothing to the shell; plain Ctrl D still ends the
   shell (EOF).
10. Click session rows to switch; close one pane (pane menu > Close, or
    `exit`); close the active session with its cross and with Ctrl Shift W;
    close a background session; middle-click a row.
    Expected: the clicked row becomes active and keeps its shell state;
    closing a pane focuses a sibling; closing the active session activates the
    session before it (the last one if it was the first); a background close
    keeps the active one. After the last session closes, the empty state
    shows (step 11). Selected row uses the selection color, hover is lighter.
    Both themes.
11. Close every session.
    Expected: the main area shows the centered card with the logo, New
    terminal and New launch, both with their shortcut text; Sessions says "No
    open sessions."; the status bar shows the host name and "OTTY <version>".
    New terminal opens a terminal; New launch opens the launch editor. Both
    themes; the card stays readable in a narrow window.
12. Run launches: a command launch (for example `htop`) from the Launches
    tree and from the chevron menu; an SSH launch with a passphrase-protected
    key.
    Expected: a command launch opens a row under "this computer" and the tree
    row shows "running" while it starts; right-click > Kill cancels it. With
    "After exit: Close" the session closes on exit; with "Keep open" the pane
    shows the exit code and closes on a key press. An SSH launch shows a row
    under its host with the "ssh" note and an SSH dot in the pane chip and
    status bar. The passphrase dialog ("Passphrase for <key>") opens centered
    with a shadow and the input focused: a wrong passphrase shows "Wrong
    passphrase. Try again."; the right one opens a logged-in session; Cancel
    closes the dialog and opens no session; an empty submit does nothing. For
    password login the title reads "Password for user@host:port". Both themes.

## Launches

13. Create a launch from the Launches "+" and from the chevron menu (New
    launch...), fill Title and Program, Save.
    Expected: the launch editor opens as a session row ("editor"); after Save
    the row closes and the launch is in the tree. An empty title or program
    shows an error and does not save. The "After exit" hint is readable.
    Both themes.
14. Edit a launch (right-click > Edit), change a field, Save; edit an SSH
    launch and switch the type.
    Expected: the editor opens with the saved values; the change is kept.
15. Rename a launch and a folder (right-click > Rename, type, Enter).
    Expected: the row shows the new name; a name used by a sibling shows "A
    sibling with this title already exists." and is not applied.
16. Duplicate a launch twice; remove a launch; delete a folder.
    Expected: two copies with unique titles; removed nodes are gone. In the
    menu, Remove and Delete are red and stay readable on hover. Both themes.
17. Create a folder (Launches folder button or the palette's New folder).
    Expected: the inline name field starts in the tree.
18. Drag a launch into a folder, then to the empty area below the tree.
    Expected: the folder highlights as a drop target; the launch moves into
    it, then back to the root. Dropping a folder into its own child is
    refused.
19. Restart OTTY.
    Expected: the launch tree (names, order, folders, open or closed folders)
    is restored.

## Files

20. Open the right panel, then `cd` to another folder in the active shell.
    Expected: the Files root follows the folder after the prompt returns, the
    folder in the pane header and status bar changes too. Switching to
    another terminal session moves Files to that session's folder; switching
    to a launch editor keeps the last folder. While `nvim` runs in the focused pane
    the line says "follows nvim".
21. Click a file in Files.
    Expected: a new session opens with the file in the default editor;
    quitting the editor closes the session.

## Command palette

22. Press Ctrl Shift K with a terminal focused; also click the title bar
    search box.
    Expected: the palette opens under the title bar with a shadow and the
    input focused; typing goes to the palette and nothing reaches the shell;
    the window does not move on the search box click. With no sessions the
    box reads "Search launches and actions". Both themes.
23. Type, move with Up and Down, run with Enter; try Esc and a click outside;
    click the palette background.
    Expected: the list filters as you type with the match in bold; Enter on a
    session switches to it and focuses its terminal; on a command or SSH
    launch it runs like a tree click (passphrase dialog included); each action
    does what its button does (New terminal, New launch, New folder, Settings,
    Split pane right, Split pane down, Close session, panels, focus mode). Esc and a click outside
    close it and the terminal gets keys again; a click on the palette
    background keeps it open. Opening the palette while a context menu is open
    only closes the menu; press the shortcut again.

## Settings, theme and language

24. Open Settings with the gear, with Ctrl , and from the palette, in Terminal
    mode and in Docker mode.
    Expected: a popup over the window with Appearance, General and Terminal
    sections, a close cross in its title, and Reset and Save under them. The
    mode does not change, no row appears in Sessions, and the title bar and the
    window buttons stay usable. Reset and Save are off until something changes.
    While it is open, the gear, Ctrl , and other shortcuts do nothing, and typed
    keys never reach the shell under it.
24b. In Settings, change Shell, then close the popup with Esc; open it, change
    Default editor and close it with the cross; open it, click in the Shell field
    and press Esc; open it, change Shell and click the dimmed area beside the
    popup (left, then right, then middle button, opening it each time).
    Expected: each time the popup closes, the change is gone when it opens again,
    and in Terminal mode the active pane takes typing at once. Change a field and
    press Save: the popup stays open, and Reset and Save turn off.
    Press in the Shell field, drag outside the popup and let go.
    Expected: the popup stays open. Open the Language list and pick a language,
    also where the list lies over the dimmed area: the popup stays open.
    While Settings is open, start an SSH launch that asks for a password.
    Expected: the password prompt shows over Settings and takes the typing. A
    click on the dimmed area beside the prompt closes neither of them.
25. Change Default editor or Shell, press Save, restart OTTY.
    Expected: the saved value is there after the restart. The "Even out
    sibling pane widths" switch also saves.
26. Switch theme between OTTY Dark and OTTY Light and press Save; then switch
    the language (for example Deutsch) and Save; switch back to English.
    Expected: the whole window and the terminals change colors at once;
    menus, the palette, panel titles, state words, the focus note and the
    empty card change language at once; no restart. Walk the Settings popup
    itself in Light.

26a. Open a new launch (Command and SSH), and press Save on the empty launch so
    the field errors show. Drag the panel edges until the main area is about 400 px
    wide. Do it in English, German and Russian. Also open an SSH launch that asks
    for a passphrase.
    Expected: the launch editor keeps its width below about 592 px, the one minimum
    of every screen; no label wraps letter by letter. Thin bars show on the right
    and at the bottom and move nothing; there is no other bar. Cancel and Save scroll
    with the fields both ways and work. Inputs and switches work while the form is
    scrolled. Wide again, the form is centered and the passphrase dialog looks as
    before.
    Scroll the launch editor, then switch to a second launch editor, and back.
    Expected: each tab starts at its top left, what you typed is still there, and no
    field has the focus. Type in a field or press Save on the empty launch while
    scrolled: the form stays where it is.
    Make the window large.
    Expected: in the launch editor the buttons sit at the bottom right of the area
    with a line above them, in the same place as Cancel and Run in Run container,
    with the main action last.
26c. Open Settings in a large window, then make the window narrow and low. Do it
    in English, German and Russian.
    Expected: in a large window the popup is centered over the main area, 640 px
    wide, with Reset and Save at its bottom right, Save last. In a small window it
    keeps its width and scrolls both ways; Reset and Save come after the last field
    and you scroll to reach them. Theme, Language, inputs and the switch work while
    it is scrolled, and the Language list opens and picks over the buttons. Type in
    a field while scrolled: the popup stays where it is. Close it scrolled and open
    it again: it starts at its top left.

26b. Close every session so the start card shows. Make the window large.
    Expected: the card is centered and looks as before, with no bars.
    Drag the panel edges until the main area is about 300 px wide and make the
    window low. Do it in English, French and Russian.
    Expected: the card keeps its 460 px, centered in a width of about 592 px, so it
    starts about 66 px in from the left, and thin bars show on the right and at the
    bottom. It scrolls both ways, New terminal and New launch can both be reached
    and pressed, and Ctrl Shift T (Cmd T on macOS) still opens a terminal.
    Dragging a panel edge across the card moves the panel and scrolls nothing;
    dragging a bar scrolls and moves no panel.
    Scroll the card, then press New launch.
    Expected: the launch editor starts at its top. Close it: the card starts at its
    top. Scroll the card again and switch to Docker mode with no engine: the
    no-engine card starts at its top.

## Keyboard and input method

27. In zsh, run `cat -v` in an OTTY terminal and press Ctrl B, Ctrl K, Ctrl W
    and Ctrl \.
    Expected: it prints `^B`, `^K`, `^W` and `^\`; nothing else happens.
    Then press Ctrl Shift T, Ctrl Shift W and Ctrl , with the terminal focused
    and with a launch editor row active: each runs its app action.
28. IBus note (GNOME): press Ctrl Shift E and Ctrl Shift O.
    Expected: nothing reaches the shell. If IBus takes Ctrl Shift E (emoji)
    before the app sees it, note it; Files uses Ctrl Shift O on Linux for this
    reason. With a non-Latin keyboard layout the letter shortcuts do not match
    (known, as split pane).
