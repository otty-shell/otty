# Regression Checklist: Modes and Docker

Manual checks for the mode switcher and Docker mode (spec AC1 to AC12) that
only a person with a real machine can do. Walk them on a release-like build
(`cargo run -p otty --release`), once in OTTY Dark and once in OTTY Light.
Shortcuts are for Linux; on macOS use Cmd.

Record each walk in the ticket's `## Comments` as
`Checklist: <date>, <commit>, <theme>, steps 1-N pass` and list any failing
step by number with what happened.

Setup before the first walk:

- Docker running, with your user in the `docker` group.
- Podman installed. Start its API with `systemctl --user start podman.socket`.
- Throwaway objects only. Every action below runs on objects named
  `otty-fx-*`, never on your own containers. Create a few first, for
  example: `docker run -d --name otty-fx-web busybox sleep 3600`, a stopped
  one (`docker create --name otty-fx-created busybox`), one that exits with 3
  (`docker run --name otty-fx-exit3 busybox sh -c 'exit 3'`),
  `docker volume create otty-fx-data`, `docker network create otty-fx-net`.
  Repeat with `podman` for the Podman part.
- A second terminal for `docker ...` commands.

## Engines and lists

1. Open Docker mode (the box icon in the switcher). Compare each table with
   the command line: Containers with `docker ps -a`, Images with
   `docker images`, Volumes with `docker volume ls`, Networks with
   `docker network ls`.
   Expected: the same items, names and counts; running containers say
   "Running"; the left panel counts match.
2. Start the Podman API (see Setup) and press Refresh.
   Expected: a second engine "Podman x.y" shows in the tree, open, with its own
   lists next to Docker's; they match `podman ps -a`, `podman images`,
   `podman volume ls`, `podman network ls`, and the counts of both engines
   change when you change something from the command line. The status bar
   names only the engine of the list on screen, never the other one.

   Press the Podman root, then the Docker root.
   Expected: each opens and closes on its own, and the table does not change.
   With the engine of the shown table closed, the table stays and no row is
   highlighted; an image link in the inspector opens that engine again.

## Actions on `otty-fx-*` objects

3. On a stopped `otty-fx-*` container: Start, Stop, Restart, Pause, Unpause,
   from the inspector buttons and from the row "..." menu. Then Remove.
   Expected: each action changes the state within a second or two, the table
   and inspector update, Remove asks first, a refused action shows a notice
   with the engine's message. Do the same on Podman.
4. Images: Pull an image (for example `busybox:1.36`), Pull again, Remove one
   that is not used. Volumes: Create `otty-fx-v`, Remove it. Networks: Create
   `otty-fx-n`, Remove it. Prune each list (it asks first; the volume prune
   shows its "named volumes too" switch).
   Expected: matches `docker images`, `docker volume ls`, `docker network ls`
   afterwards. A used volume cannot be removed and says why.
4a. Image tags. Use throwaway names: `docker tag <an image of yours> otty-fx-a:1`
   and `docker tag <the same image> otty-fx-a:2`.
   Expected: two rows `otty-fx-a` with the tags `1` and `2`, the same ID, size
   and "Used by"; the tree count and `docker images` agree. Selecting one
   highlights only that row and the inspector title names it. `Tag:2` in the
   search keeps that row. Remove on `otty-fx-a:1` says only the tag goes, has no
   Force switch and leaves your image and the other tag; remove `otty-fx-a:2`
   the same way.
4b. Force. Build a throwaway image, run a container from it, stop the container
   and press Remove on the image's only tag.
   Expected on Docker: Force is on, and the dialog says the tag goes and the
   image stays without one. Turn Force off: the dialog says the engine refuses,
   and pressing Remove shows the engine's message in a notice. Expected on
   Podman: Force is off, and the dialog says the engine refuses; pressing Remove
   shows the engine's message in a notice and the container stays. Turn Force
   on: the dialog says the stopped container goes too. With
   the container running on Podman: no switch, and the dialog says to stop it
   first.

## Run form

5. Containers, Run container. Image `nginx:alpine`, name `otty-fx-run`, port
   host `18090` to container `80`, a bind mount of an empty folder to
   `/usr/share/nginx/html`. Run.
   Expected: the form closes, the new container is selected and running.
   `curl localhost:18090` answers. `docker inspect otty-fx-run` shows
   `PortBindings` `80/tcp` to `18090` and a bind in `Mounts` with the folder
   and destination you typed. Same on Podman with `podman inspect`.
6. On Fedora (SELinux on): repeat step 5 with the SELinux switch on the
   mount, then off.
   Expected: with the switch on, `Binds` ends with `:z` and files in the folder
   can be read in the container; with it off there is no `:z`.
7. Open the form and type a port `99999`, an empty container path, a missing
   folder.
   Expected: each field shows its error, Run stays disabled and the footer
   says to fix the marked fields. The status bar says "Run container" while
   the form is open.
8. Run an image you do not have (for example `hello-world:latest` after
   removing it).
   Expected: a pull line with progress shows, then the container starts.

## Shell

9. Select a running `otty-fx-*` container, press Shell in the inspector; then
   the same from the row "..." menu.
   Expected: OTTY switches to Terminal mode with a new tab running
   `docker --host <address> exec -it -- <id> sh`, where `<address>` is the
   address of that container's engine; exiting the shell closes it normally.
   On Podman the tab runs `podman --url <address> exec -it -- <id> sh`.

## Live updates

10. With Docker mode open on Containers, run `docker stop otty-fx-web` and
    `docker start otty-fx-web` in another terminal.
    Expected: the row changes state within about a second without pressing
    Refresh. Create and remove a container from the terminal: the row appears
    and goes away.
11. Podman idle: open Docker mode with Podman only, leave it for 2 minutes
    without touching anything, then `podman stop` a container.
    Expected: the table still updates (the events stream does not drop while
    idle), or after a break the table catches up by itself. Report any
    moment the table stays stale.

## No engine

12. `sudo systemctl stop docker docker.socket`, open Docker mode (or press
    Check again).
    Expected: card "Docker is installed but not running" with the start
    command and its copy icon; the left panel and status bar say so too.
13. Stop Podman only: `systemctl --user stop podman.socket` (with no Docker).
    Expected: card for the Podman API being off with the `systemctl --user
    start podman.socket` command.
14. Use a user that is not in the `docker` group (log in as one, or run
    `sg` / a test account with the socket owned by root:docker).
    Expected: a "no access" card naming the socket and the group command.
15. A machine or container with neither command.
    Expected: card with the Docker and Podman install tiles and no command.
    After each case start the service and press Check again.
    Expected: the engine appears and the lists load. The "OTTY looked for"
    line lists the sockets tried.
15e. With both commands installed, stop Docker and Podman
    (`sudo systemctl stop docker docker.socket`,
    `systemctl --user stop podman.socket podman.service`) and open the mode.
    Expected: card "Docker and Podman are installed, but neither is answering"
    with both commands, each under its own label with its macOS note. Each copy
    icon puts its own command on the clipboard, and How to fix Docker and How to fix
    Podman open their own pages. All of it fits in a window at its smallest
    size, in English and in German. With only one command on `PATH`: only that
    command, as steps 12 and 13.
15i. With a user outside the `docker` group and Podman installed, stop Podman
    (`systemctl --user stop podman.socket podman.service`) and open the mode.
    Expected: card "OTTY has no access to Docker, and Podman is off" naming the
    socket, with the docker group command and the Podman command, each under its
    own label with its note. Each copy icon puts its own command on the clipboard, How
    to fix Docker opens the docker group page and How to fix Podman the Podman
    page. The left panel says "No access to Docker, and Podman is off." and the
    status bar names the socket and says the Podman socket is off. Run the Podman
    command and press nothing.
    Expected: within about 7 seconds the Podman table replaces the card. With
    Podman removed or hidden from `PATH`: only the docker group card, as step 14.
    Mirror, if you have a Podman socket you may not open
    (`/run/podman/podman.sock`) and Docker installed and stopped, and your own
    Podman socket off: card "OTTY has no access to Podman, and Docker is not
    running" with the Podman command first and `sudo systemctl start docker`. With
    a `docker` command that is really Podman (`podman-docker`): the Podman card
    only, with no word of Docker. Check that the card fits a window at its smallest
    size, in English and in German: the "looked for" line may touch the bottom edge.
15j. On a card with `systemctl --user enable --now podman.socket`, in OTTY Dark
    and OTTY Light, with the window large.
    Expected: the card is centered both ways, 460 px wide, with no bar. Drag the
    panel edges so the main area narrows slowly.
    Expected: down to about 592 px there is no sideways bar and the card stays in
    the middle. Below that the card keeps its 460 px, a thin bar shows at the
    bottom of the area, and the card stays in the middle of those 592 px, so at the
    narrowest it starts about 66 px in from the left. The whole command shows on one
    line, with no bar under it. The drag itself never scrolls the card. Shift and
    the wheel, a sideways wheel or a touchpad, or dragging the bar, scroll the
    area; press the copy icon while scrolled and the whole command is copied. Widen
    the area again: the bar goes away and the card centers again. In Korean, check by eye that the text reads as whole syllables
    (see step 36b).
15k. On the card with two commands (15e) make the window as low as it goes, and
    drag the panels wide.
    Expected: while the card fits, it is centered with no bar. When it does not,
    it starts at its top with a thin bar on the right of the area. The wheel
    reaches Check again, each How to fix and the "looked for" line, also with the
    pointer over a command box, and each button works. Do the same on the card
    with the install tiles (15) and on a card with one command (12), in English,
    German, Russian and Korean. Then, scrolled down and to the right, wait 20
    seconds.
    Expected: the search behind the card does not move it, nor make it blink.
    Start an engine: the table replaces the card. Stop it: the card is back, at its
    top. Scroll it again and press Check again: the card returns at its top.
15f. With a card on screen, start an engine from a terminal and press nothing.
    Expected: within about 7 seconds the table replaces the card. Stop it again.
    Expected: the card comes back, and it does not blink or turn into "searching"
    while you watch it for 20 seconds. Press a copy icon, and How to fix, while it shows:
    they work as always. Press Check again: "searching" shows until that search
    answers.
15g. With Docker as the only engine, open Images, select the second row, stop
    Docker and start it again (Check again, or wait).
    Expected: Images is open and that row is selected. Do the same with a row of
    Containers on the Logs tab.
    Expected: that container is selected and its last lines show. On Podman as
    the only engine, select a throwaway `otty-fx-*` container, stop
    `podman.socket` and `podman.service`, run `podman rm -f` on it and start the
    socket.
    Expected: the first row is selected.
15h. With Docker and Podman on, show a Docker list, stop Docker and start it
    again.
    Expected: the view moves to Podman and stays there when Docker comes back.

15a. With Docker as the only engine and its list on screen, stop it from a
    terminal: `sudo systemctl stop docker docker.socket`.
    Expected: within a few seconds the card "Docker is installed but not
    running" replaces the table, the tree has no rows and the status bar says
    so. Start it again and press Check again.
    Expected: the engine shows open in the tree with its table on the right.
    Do the same with Podman as the only engine
    (`systemctl --user stop podman.socket`): the Podman card and its command.
15b. Press the engine root so it is closed, stop the engine and wait for the
    card. Start it again and press Check again.
    Expected: the table shows its containers, the tree root stays closed and no
    row is highlighted. Pressing the root opens it.
15c. With Docker and Podman on, close the Podman root, show a Docker list and
    stop Docker.
    Expected: the table moves to Podman's containers and the Podman root opens,
    so you can see whose list it is.
15d. Open the Run form and type an image, then stop the only engine.
    Expected: the card replaces the form. Start the engine and press Check
    again.
    Expected: the form is back as you typed it.

## Copy, links, Settings

16. Hold the pointer on the copy icon of a command box.
    Expected: the tooltip says Copy, in the language of the app. Press the icon
    and paste into a terminal.
    Expected: the exact command is pasted. Install tiles and How to fix open
    the right pages in the browser.
17. In an Overview, press a port link, the Compose name, an image link and a
    network link.
    Expected: the port opens `http://localhost:<port>` in the browser; the
    others select that item in its list or filter the containers by project.
    The same port links work from the Ports column of the table and do not
    select the row.
18. Open Settings (title bar gear) from Docker mode.
    Expected: Terminal mode shows with the Settings tab.

## Keys

19. In a terminal tab run `cat -v`. Press Ctrl 1 and Ctrl M.
    Expected: `cat -v` prints the control characters (`^M` for Ctrl M); the
    keys reach the shell and do not change mode. Then press Ctrl Shift 2.
    Expected: the mode switches (Docker), and Ctrl Shift 1 returns to Terminal.

## Logs, Inspect and narrow window

20. Select a container that prints output (for example
    `docker run -d --name otty-fx-ticker busybox sh -c 'while true; do date; sleep 1; done'`),
    open Logs.
    Expected: the last lines show and new ones follow. Turn Follow off:
    lines stop moving and "N new lines" counts up; select some text and
    copy it. Turn Follow on: the waiting lines appear and the view follows.
20a. Logs across a broken connection. On Podman, with `podman.socket` running
    and the logs of a ticking `otty-fx-*` container open, stop and start only
    the service; the socket stays, so the search still finds the engine:
    `systemctl --user stop podman.service; sleep 3; systemctl --user start podman.service`.
    Expected: within a few seconds the lines go on. Nothing is cleared, the
    lines of the gap are there and no line shows twice (compare with
    `podman logs`). The same with Follow off.
    Expected: the view stays and the counter grows by the lines of the gap;
    turning Follow on adds them in order.
    Stop the service, run `podman restart <name>`, start the service.
    Expected: the view is cleared once and shows only the new run.
    Stop the service, run `podman stop <name>`, start the service.
    Expected: the lines stay and nothing new comes; no error stays under them,
    also none that shows up a little later, and the logs do not reload over and
    over.
    With Podman as the only engine, stop the socket as well, then start it
    and press Check again.
    Expected: the card while it is away; afterwards the same row is selected
    and the open Logs tab shows its last lines.
21. Inspect tab on a container, an image, a volume and a network.
    Expected: General first, then State, Config and the others as sections;
    sections open and close; Show empty adds empty values; Raw JSON shows
    the engine's JSON; Copy JSON puts it on the clipboard. Values stay clear
    of the scrollbar.
22. Make the window about 900 px wide.
    Expected: the toolbar buttons that do not fit move into "..."; the
    inspector keeps readable values (key and value stack when narrow); no
    text is drawn outside its panel. Look at both themes.

23. Make the window short, so a Docker table scrolls. Drag the `main | right
    panel` split by its line and the first pixels right of it.
    Expected: the pointer shows the resize shape there, the panel follows, the
    table does not scroll, and nothing lights up under the pointer on the way.
    Then drag the table's scrollbar, right beside the line.
    Expected: the bar lights up, the table scrolls and the panel stays put, even
    when the pointer goes over the line during the drag. Do the same with the
    left panel, Sessions or the Files tree having a scrollbar in Terminal mode.
24. Split a terminal tab side by side and run `seq 1 200`. Drag the split by its
    line.
    Expected: the panes resize; no text gets selected and the program gets no
    mouse events. Then select text in the left pane up to its last column, and
    let go on the line.
    Expected: the selection follows to the end and ends there; no resize starts.
24a. Open Run container and add a port and a volume row. Drag the panel edges
    until the main area is about 400 px wide, then back. Do it in English, German
    and Russian.
    Expected: down to about 592 px the form shrinks as before; this is the one
    minimum of every screen (Decision 23). Below that it keeps its width, no label
    wraps letter by letter, and thin bars show on the right and at the bottom,
    moving nothing. The drag itself never scrolls the form. There are
    only these two bars: the hint, Cancel and Run are part of the form, after its
    last field, and scroll with it both ways. Shift + wheel (or a sideways swipe)
    over the form or over the buttons scrolls it sideways, the wheel scrolls it
    down, and each bar can be dragged. Run and Cancel work while it is scrolled,
    and on the waiting, folder and fix-the-fields hints the text wraps beside the
    buttons at the smallest width in German and Russian.
    With the window large and few fields, the buttons sit at the bottom of the
    area with a line above them. Make the area about 1400 px wide, here and in
    Settings and the launch editor (Command and SSH).
    Expected: the form stays centered at 720 px. The line spans the area, and the
    buttons end at the right edge of the fields, not of the area. In Run container
    the hint starts at the left edge of the fields. In Settings, Reset has the look
    of Cancel: plain text, a box under the pointer, dimmer while there is nothing
    to reset. At about 700 px the form fills the area and the buttons end at its
    right edge, with the side padding.
    In the volume row of Run container, in Russian, Portuguese and French at any
    width.
    Expected: the read-only label reads whole, with no "…", also in Korean.
    With the area at its narrowest, in German and Russian, here and in the launch
    editor (Command and SSH).
    Expected: the placeholder of the command box reads whole, also behind a token.
    The word beside Name and Command is in that language (German spells it
    "optional" too). Add port, Add variable, Add volume and the launch editor's add
    button have one plain look.
    Add rows until the form is taller than the area:
    the buttons move down with the form, and you scroll to reach Run. Start a pull
    of a long image name with the area at its narrowest.
    Expected: the pull line wraps beside Cancel and Running, and Cancel works.
    Type in the Image field, select text in it, open the Network list and pick
    an item while the form is scrolled.
    Expected: everything works and the list opens under its field. Type a wrong
    port: the message under the row wraps and pushes the next row down.
    Scroll an input you are typing in out of view and go on typing.
    Expected: the text goes into that input, and the view does not jump to it
    (known limit). With the form scrolled, type in a field, add a row and type a
    wrong port.
    Expected: the form stays where it is. Scroll it, stop the only engine, then
    start it again.
    Expected: the card starts at its top, and the form comes back at its top with
    what you typed.
24b. Drag the right panel to its smallest while Run container is open, with an
    image on this computer, then one that is missing, in English, German and
    Russian, in OTTY Dark and OTTY Light.
    Expected: the whole status under the image name can be read, on as many lines
    as it takes, with its dot beside it. Make the window low: the thin bar runs along
    the edge of the panel and covers no text. Widen the panel: the status is one line
    again.

## Tag search

Use the search field above a Docker table, with a few containers (names,
statuses) in the list.

25. Click the field, type `sta`.
    Expected: the first row is `sta` itself, drawn muted (not a real
    suggestion), then the matching keys, then the matching names. Press Tab,
    then type `r`.
    Expected: Tab completes to the first real suggestion, so the field shows
    `Status:` with the cursor after the colon and the values listed; the next
    letter gives `Status:r`, not `rStatus:`. Repeat with ↓ then Enter, and with
    a click on the `Status` suggestion. The field keeps focus after the click.
    With an empty field, or with only `Status:` typed, there is no muted row
    and the first real suggestion is highlighted.
26. Type `Status:ru`.
    Expected: the first row is `Status:ru` itself, then the matching values.
    Press Enter.
    Expected: a `Status:ru` tag appears, the field is empty and still focused,
    and the table keeps every row whose status contains `ru`. Then type
    `status:ru` (the case of the key does not matter) and press Tab, or press
    ↓ then Enter, or click a value.
    Expected: a `Status:Running` tag appears. Type a name such as `web` and
    press Enter.
    Expected: the text `web` stays in the field, the table is filtered by
    name and the list closes. Type `redis:7` or `foo:bar` and press Enter.
    Expected: the same, as a name filter. Type `web`, press Tab, then type `x`.
    Expected: Tab completes to the first real name; the cursor is after it
    (`...x` after the name).
27. Type two letters, press Left, type a third letter.
    Expected: the letter lands between the two; the cursor does not jump to
    the end. Up and Down do not move the cursor either.
28. Add tags, type some text, switch to another list (for example Images) and
    back.
    Expected: the tags and the text are still there for each list and the table
    is filtered by them. The suggestion list is closed on the way back and opens
    when you click the field.
28b. Type in a Docker search so its list opens, then switch to Terminal mode
    (a click on the switcher, or its shortcut) and back to Docker.
    Expected: the text and the tags are still there, and the list is closed until
    you click the field.
29. Remove one tag with its cross, then press Clear all. Press Backspace in
    an empty field.
    Expected: the cross removes only its tag; Clear all removes all; Backspace
    removes the last tag.
30. In the Containers table press a project name in the Compose column, then
    press another project's name.
    Expected: the first press adds the tag `Compose:<name>` and filters the
    table; the second replaces that tag. The row is not selected by the press.
31. Type a word with a typo, such as `stauts`.
    Expected: the suggestions still list the close matches (`Status`).
32. Open the suggestions with the list below the field.
    Expected: the list floats over the tags row and the table, not pushing
    them down; a click outside closes it and also reaches what is there.
33. With a list of more than eight suggestions, press Down past the eighth,
    then Tab, then Esc, then use the mouse wheel on the list.
    Expected: the list scrolls to keep the highlighted row in view; Tab
    applies the highlighted real row (Tab on the muted typed-text row applies
    the first real row below it); Esc closes the list; the wheel scrolls it
    without changing the highlighted row.
34. Switch the interface to Russian in Settings. In the Containers table focus
    the search field.
    Expected: the list shows the column headers (`Имя`, `Состояние`, `Образ`,
    `Compose`, `Порты`, `Возраст`). Type `Сост`, press Tab.
    Expected: the field shows `Состояние:`. Type `раб`, press Tab.
    Expected: a tag `Состояние:Работает` appears and only running containers
    stay. Type `имя:otty` and press Enter, then type `status:x`.
    Expected: `имя:otty` gives the tag `Имя:otty`; `status:x` is not a key, so
    the whole text filters by name and no row matches. Clear the field and type
    `Состояние：раб` with a wide colon.
    Expected: the first row reads `Состояние:раб`. In the Images table `Реп`
    then Tab gives `Репозиторий:`.
35. With the tag `Состояние:Работает` still there, switch the interface back to
    English.
    Expected: the tag reads `Status:Работает` and, as the owner accepted, matches
    no row until it is removed, because the cell now says `Running`. A press on
    a project name in the Compose column still adds and replaces the `Compose`
    tag in both languages.

35a. With Docker and Podman on, add a tag and press a column header on Docker's
    Containers, then open Podman's Containers.
    Expected: no tag there and its own sort. Add another tag and sort there, go
    to Docker's Images and to Podman's Images, then back to both Containers
    pages.
    Expected: each page has the tags, typed text and sort it had. Suggestions on
    a page list only that page's rows. A press on a project name adds the
    `Compose` tag on that engine's page only.
35b. With a tag, typed text and a sort on Docker's Containers, stop Docker and
    start it again (`Check again` if the card shows).
    Expected: the tag, the text and the sort are still there, and the suggestion
    list is closed even if it was open when Docker stopped. Restart OTTY.
    Expected: they are gone.
35c. Create names that differ in case: `docker volume create otty-fx-Beta`,
    `docker volume create otty-fx-alpha`, `docker volume create otty-fx-Gamma`.
    In Volumes sort by Name.
    Expected: `otty-fx-alpha`, `otty-fx-Beta`, `otty-fx-Gamma`, and the reverse
    on a second press. Do the same with image tags in the Tag column: `<none>`
    comes after tags that start with a digit and before tags that start with a
    letter.

35d. Look at the mode's name.
    Expected: the switcher tooltip, the All modes menu and the palette ("Switch to
    Containers mode") say Containers, also in Russian and in Japanese, and so does
    the status bar while no engine is shown. The tree root, the cards and the
    status bar next to a table still say Docker or Podman. The icon has not
    changed.

35e. Create containers with ports:
    `docker run -d --name otty-fx-p80 -p 18000:80 nginx:alpine`,
    `docker run -d --name otty-fx-p9 -p 9000:80 -p 19000:81 nginx:alpine`, and
    keep `otty-fx-web` from the setup, which has no ports. In Containers press
    the Ports header.
    Expected: the arrow shows on Ports, pointing up, and the rows read
    `18000:80/tcp`, then the two ports of `otty-fx-p9` in the order its cell
    shows them, then the rows that show `—`, by name. This is text order, so
    `18000` comes before `9000`. A second press reverses it, with the `—` rows
    first and still by name among themselves.
    Press a port link in a sorted row.
    Expected: that row's port opens in the browser and the row is not selected.
    Narrow the window until the table scrolls sideways.
    Expected: the Ports header can still be pressed and the column keeps enough
    room for one port. Remove the containers with
    `docker rm -f otty-fx-p80 otty-fx-p9`.

## Both themes

36. Walk steps 1, 3, 5, 12, 20 and 25 in OTTY Light as well.
    Expected: every text, badge dot and icon is easy to read on its surface;
    the selected row, hovered row and the row "..." menu all keep clear
    contrast.
36a. In OTTY Light make the window small, and look at the thin bars without
    touching them: a table on Containers, the left tree, a Run form, the no-engine
    card, and a command box with the panels dragged wide.
    Expected: each idle bar is plainly visible and quieter than the text. With the
    pointer on a bar, or dragging it, the bar is clearly darker. In OTTY Dark the bars
    look as they did.

## Chinese, Japanese and Korean text

36b. Set the language to Korean in Settings and press Save. Look at a Run form with a
    folder row at the narrowest width, a card with two commands, a table and the
    left tree, in OTTY Dark and OTTY Light.
    Expected: the text reads as whole syllables, not separate letters, and headings
    and buttons are clearly bolder than the text under them. The read-only label
    and the SELinux hint do not wrap letter by letter. Then run `echo 읽기` in a
    terminal.
    Expected: the syllables are whole and stay within their cells.
36c. Save a launch titled `直骨令`, so the left panel shows it, and run `echo 直骨令` in
    a terminal. In Settings switch to Japanese and Save, then to Traditional Chinese
    and Save, without restarting the app. On these, 骨 and 令 look different in each.
    Expected: the letters change shape with the language at once, in the panel and
    in the terminal. In Japanese the kanji and the kana read as one font, also in
    headings. On a computer with no CJK font these languages show boxes; that is
    accepted.
