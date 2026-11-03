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
   change when you change something from the command line. The status bar says
   "Podman ... also found" when the table shows a list of the other engine.

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
   `docker exec -it <id> sh`; exiting the shell closes it normally.

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
    command and Copy; the left panel and status bar say so too.
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

## Copy, links, Settings

16. Press Copy on a command box and paste it into a terminal.
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
28. Add tags, switch to another list (for example Images) and back.
    Expected: the tags you added are still there for each list.
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

## Both themes

36. Walk steps 1, 3, 5, 12, 20 and 25 in OTTY Light as well.
    Expected: every text, badge dot and icon is easy to read on its surface;
    the selected row, hovered row and the row "..." menu all keep clear
    contrast.
