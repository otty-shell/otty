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
   Expected: a second engine "Podman x.y" shows in the tree with its own lists;
   they match `podman ps -a`, `podman images`, `podman volume ls`,
   `podman network ls`. The status bar says "Podman ... also found" when
   the other engine is not the open one.

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
   afterwards. A used image or volume cannot be removed and says why.

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

## Both themes

25. Walk steps 1, 3, 5, 12 and 20 in OTTY Light as well.
    Expected: every text, badge dot and icon is easy to read on its surface;
    the selected row, hovered row and the row "..." menu all keep clear
    contrast.
