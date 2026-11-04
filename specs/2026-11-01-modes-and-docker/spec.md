# Modes and the Docker Mode

Created: 2026-11-01
Author: holland.deception@gmail.com
Agent: Claude Code
Status: IN PROGRESS
Approved: Yes
Worktree: No
Type: Build

## Summary

**Goal:** Let OTTY switch between ways of working ("modes"). Terminal is the
default mode and stays as it is today. The first new mode is Docker: see and
manage containers, images, volumes and networks of the local Docker or Podman
engine in tables, with an inspector on the right. Later modes (Kubernetes,
agent work) plug in the same way.

The mode switcher sits at the top of the left panel. Pinned modes are icon
tabs; after them a "⋯" button opens the All modes menu, where every mode can
be opened, pinned or unpinned.

**References:**

- Screens: the redesign canvas (https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6), page "v4 today", row "Where to switch between Terminal, Docker and Kubernetes":
  - `V4ModeSwitch`, variant `panel-icons` ("Option A: mode icons at the top of the left panel"): switcher, Docker tree, table, inspector.
  - `V4ModesMany` ("Option A with many plugins: pinned modes and the All modes menu"): pinned tabs, temporary tab, All modes menu.
  - `V4FutureModes`: the Docker tree, columns and inspector fields in more detail.
- Screens for this spec: the same canvas, page "Docker mode" (drawn 2026-11-01). These win over the "v4 today" boards above:
  - `DockerMode` (interactive): switcher, engine tree, Containers table, inspector with Overview / Logs / Inspect, remove dialog, error notice. Tweaks: `resource`, `menu`, `confirm`, `notice`, `theme`.
  - `DockerModesMenu`: the All modes menu open; Pin / Unpin works.
  - `DockerImages`, `DockerVolumes`, `DockerNetworks`: the other tables and their inspectors, including disabled Remove.
  - `DockerRun` (interactive): the Run container form. Tweaks: `state` (ready, pulling, error), `invalid`.
  - `DockerNoEngine`, `DockerEngineStopped`, `DockerPodmanOff`, `DockerNoAccess`: the four Decision 9 cases.
- Design system (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3).
- Engine client: [bollard](https://github.com/fussybeaver/bollard) 0.21.1 (Apache-2.0). It speaks the Docker Engine API, which Podman also serves (`podman system service`), and has `Docker::connect_with_podman_defaults()` for Podman sockets.
- Previous spec: `specs/2026-10-31-workspace-layout` (left/right panels, kit-first rule, shortcut rules; done).

Where a screen and this spec disagree, the Decisions section wins. The screens
show plugins, Kubernetes, a bottom terminal dock and right-panel plugin tabs;
none of those are in scope (see Out of Scope).

### Current state (research, 2026-11-01)

| Fact | Location |
|---|---|
| The window is title bar, `left \| main \| right` pane grid, status bar; the sidebar widget owns both panels, widths and focus mode | `app/src/widgets/sidebar/state.rs`, `app/src/view.rs` |
| The left panel shows Sessions over Launches; the right panel shows Files | `app/src/widgets/sidebar/view/left_panel.rs`, `app/src/view.rs` (`view_left_panel`, `view_right_panel`) |
| There is no notion of a mode; the main area always shows the active tab | `app/src/view.rs` (`view_tab_content`) |
| Every widget follows `state → reducer → vm() → view(Props) → Intent`; events are routed in `app/src/events/` | `app/src/widgets/*`, `app/src/events/mod.rs` |
| Async work runs through `Task::perform`; long-lived streams through `Subscription::run_with` | `app/src/events/explorer.rs:64`, `app/src/widgets/explorer/watcher.rs:71` |
| iced runs on tokio (`iced` feature `tokio`), which bollard needs | `app/Cargo.toml` |
| One keymap table feeds both the terminal bindings and the app keyboard subscription; Linux uses Ctrl Shift, macOS uses Cmd | `app/src/shortcuts.rs` |
| Settings are saved to `~/.config/otty/settings.json` through an explicit Save button | `app/src/widgets/settings/storage.rs` |
| Kit components exist for tree rows, menus, segmented control, dialog, empty card, chip, target dot, status bar, buttons | `crates/ui/kit/src/components/` |
| The kit has no table, status badge, key-value list, log view, inline notice or mode switcher | `crates/ui/kit/src/components/` |
| Nothing opens a URL in the browser today | — |
| Supported platforms are Linux and macOS | `README.md` |
| bollard 0.21.1: default features `http`, `pipe`; Podman socket order: `$DOCKER_HOST` (unix), `$XDG_RUNTIME_DIR/podman/podman.sock`, `/run/user/$UID/podman/podman.sock`, `/run/podman/podman.sock` | bollard `src/docker.rs:975-1022` |

## Decisions

1. **A mode is a closed list, not a plugin.** `Mode` is an enum with
   `Terminal` and `Docker`. Adding Kubernetes or agent work later means a new
   variant and a new widget. No plugin API, no mode trait, no registry: there
   is one real implementation per mode and nothing outside OTTY adds modes
   today.
2. **The switcher is at the top of the left panel** (Option A). It shows:
   Terminal, then the pinned modes in the order they were pinned, then a
   temporary tab for the active mode if it is not pinned, then the "⋯" (All
   modes) button. The name of the active mode is shown next to the tabs.
   - Terminal is always first and cannot be unpinned.
   - Up to 4 modes besides Terminal can be pinned. When 4 are pinned, the
     pin button of other modes is disabled with the hint "Unpin another mode
     first".
   - Docker is pinned on first start (owner approved, 2026-11-01).
   - The temporary tab has an outline (`mode-tab-temp` on the screen) and
     disappears when another mode is chosen.
3. **The All modes menu** opens under the "⋯" button. Sections: "Pinned (N of
   4)" with Terminal and the pinned modes, each with its shortcut and an
   Unpin button (none for Terminal); "Other modes" with a Pin button. Clicking
   a name switches to that mode and closes the menu. Esc or a click outside
   closes it. No search field, no drag to reorder and no "Get more modes"
   item until there are enough modes to need them.
4. **Pinned modes are saved right away** in `~/.config/otty/modes.json`, not in
   `settings.json`: pinning is not a settings form change and must not wait
   for the Save button or overwrite unsaved form edits. A missing or broken
   file falls back to the defaults (Docker pinned) with a log warning. OTTY
   always starts in Terminal mode.
5. **A new `modes` widget owns the active mode, the pinned list and whether
   the menu is open.** The sidebar widget keeps the panels: open/closed and
   widths are shared by all modes. The root view asks the active mode what to
   put in the left panel, main area and right panel:

   | Slot | Terminal | Docker |
   |---|---|---|
   | Left panel | switcher + Sessions + Launches (today) | switcher + engine tree |
   | Main area | active tab (today) | resource table |
   | Right panel | Files (today) | Inspector of the selected item |
   | Status bar | today's items | engine name and version, item counts |

   Terminal tabs keep running while another mode is shown. So a command that
   acts on the active tab does nothing while that tab is hidden (owner
   decision, 2026-11-03): outside Terminal mode, Close session (shortcut and
   palette) and the palette's Split pane do nothing, and the palette does not
   list those two. The palette's New folder follows the same rule (owner
   decision, 2026-11-03): its name field lives in the Launches panel, which
   only Terminal mode shows, so outside Terminal mode it is not listed and a
   press from a list opened earlier does nothing. A command that opens or
   activates a tab still switches to Terminal mode. When the left panel is
   hidden (panel toggle, focus mode) the switcher is hidden too; shortcuts and
   the command palette still switch modes.
6. **Shortcuts follow the keymap rules** (no keys taken from the shell):

   | Action | Linux | macOS |
   |---|---|---|
   | Terminal mode | Ctrl Shift 1 | Cmd 1 |
   | Pinned mode 1–4 | Ctrl Shift 2–5 | Cmd 2–5 |
   | All modes menu | Ctrl Shift M | Cmd Shift M |

   Digits are matched by physical key, because Shift changes the character
   (Shift 1 is `!` on US layouts). Owner approved Ctrl Shift over Alt
   (2026-11-01). The old `V4ModeSwitch` and `V4ModesMany` boards show Ctrl
   1–5; the Docker mode boards (see References) already use this table. The
   command palette gets "Switch to Terminal mode" and "Switch to Docker
   mode".

   One known exception to "no keys taken from the shell": on US layouts
   Ctrl Shift 2 is Ctrl @, which sends NUL (`^@`, used for example by Emacs
   `set-mark`). It now switches to pinned mode 1. NUL stays available with
   Ctrl Space (owner approved, 2026-11-01).
7. **Docker logic lives in a new crate `otty-docker`** (`crates/docker`). It is
   the only place that uses bollard. Reason: it crosses a real infrastructure
   boundary (the engine socket), and bollard's generated types (every field
   `Option`) are mapped once into small OTTY types (`Container`, `Image`,
   `Volume`, `Network`, `EngineInfo`). The crate has no trait over the engine:
   the mapping and the engine discovery are pure functions tested on plain
   inputs, and the calls themselves are thin.
8. **Docker and Podman work the same way.** OTTY finds engines in this order
   and shows every engine it reaches as a root in the left tree:
   - `$DOCKER_HOST`, if set (unix socket or tcp).
   - Docker: `/var/run/docker.sock`; on macOS also `~/.docker/run/docker.sock`.
   - Podman: the rootless socket, then `/run/podman/podman.sock` (bollard's
     order); on macOS the socket of the default Podman machine.
   - Other Docker-compatible engines (owner approved, 2026-11-01): Docker
     Desktop on Linux `~/.docker/desktop/docker.sock`, Colima
     `~/.colima/default/docker.sock` (Linux and macOS), OrbStack
     `~/.orbstack/run/docker.sock` (macOS). They show as Docker.
   - The same socket found twice counts once.

   Finding the `docker` and `podman` commands (for Decision 9 and for Shell)
   also looks in the usual install folders, because an app started from the
   macOS Finder gets a short `PATH`: `/usr/local/bin`, `/opt/homebrew/bin`,
   `~/.docker/bin`, `/Applications/Docker.app/Contents/Resources/bin` and
   `/opt/podman/bin`. A `docker` command that is really Podman (the Fedora
   `podman-docker` package; `docker --version` names Podman) counts as
   Podman, so a Podman-only user sees the Podman card, not the Docker one
   (owner approved, 2026-11-01).

   Whether an engine is Docker or Podman is read from its `/version` answer
   (Podman reports a "Podman Engine" component). The root row shows "Docker"
   or "Podman", the version and the "this computer" note. Only local engines
   are in scope.
9. **When no engine is reachable, the Docker mode says why and what to do,**
   using the kit `EmptyCard` in the main area (left tree and inspector are
   empty):

   | Case | How it is detected | Text and buttons |
   |---|---|---|
   | Nothing installed | No socket answers and neither `docker` nor `podman` is on `PATH` | "Docker or Podman is not installed". Buttons: **Install Docker** (https://docs.docker.com/get-started/get-docker/), **Install Podman** (https://podman.io/docs/installation), **Check again** |
   | Docker not running | `docker` is on `PATH`, no Docker socket answers, Podman is not installed | "Docker is installed but not running", command `sudo systemctl start docker` (macOS note: open Docker Desktop), **Copy**, **Check again**, **How to fix** (https://docs.docker.com/engine/daemon/start/) |
   | Podman API off | `podman` is on `PATH`, no Podman socket answers, Docker is not installed | "Podman is installed but its API service is off", command `systemctl --user enable --now podman.socket` (macOS note: `podman machine start`), **Copy**, **Check again**, **How to fix** (https://docs.podman.io/en/latest/markdown/podman-system-service.1.html) |
   | Docker and Podman both off | Both of the two rows above hold (owner decision, 2026-11-04): each engine counts as installed by its command or by a socket of its own that does not answer, and a `docker` command that is really Podman counts as Podman | "Docker and Podman are installed, but neither is answering". Both commands, each under a label that names its engine and with its own macOS note, each with **Copy**. **Check again**, **How to fix Docker** and **How to fix Podman** (the two pages above) |
   | No permission | The socket exists, connecting fails with permission denied | "OTTY has no access to the Docker socket", command `sudo usermod -aG docker $USER` with the note to log out and back in and that the docker group has root-level access, **Copy**, **Check again**, **How to fix** (https://docs.docker.com/engine/install/linux-postinstall/) |
   | No permission, Podman socket | A Podman socket exists (found in the Podman step of Decision 8, or `$DOCKER_HOST` with `podman` in the socket's file name), connecting fails with permission denied. The docker group has nothing to do with it | "OTTY has no access to the Podman socket", command `systemctl --user enable --now podman.socket`, which turns on the user's own socket at `$XDG_RUNTIME_DIR/podman/podman.sock`, the first Podman place Decision 8 looks at, with the note that it shows the user's own containers, not root's (macOS note: `podman machine start`), **Copy**, **Check again**, **How to fix** (https://docs.podman.io/en/latest/markdown/podman-system-service.1.html) |
   | No permission, Podman off | The Docker socket refuses as in "No permission", and Podman counts as installed as in "Docker and Podman both off" (owner decision, 2026-11-04). When a Podman socket refuses too, the first denied socket decides alone | "OTTY has no access to Docker, and Podman is off", the text naming the socket and that Podman is installed but off. The docker group command with its note, then the Podman command with its macOS note, each under its label with **Copy**. **Check again**, **How to fix Docker** (the docker group page) and **How to fix Podman** |
   | No permission on Podman, Docker off | A Podman socket refuses as in the row above it, and Docker counts as installed as in "Docker and Podman both off"; a `docker` command that is really Podman does not count, so that case stays on "No permission, Podman socket" (owner decision, 2026-11-04) | "OTTY has no access to Podman, and Docker is not running", the text naming the socket and that Docker is installed but not running. The Podman command with its note, then `sudo systemctl start docker` with its macOS note, each under its label with **Copy**. **Check again**, **How to fix Podman** and **How to fix Docker** (the Docker start page) |

   **Copy** in this table is the copy icon of the kit `CommandBox`, with "Copy"
   as its translated tooltip (owner decision, 2026-11-04, as Decision 20 did for
   Copy JSON). Each command has its own icon. A command box is one line high on
   every card, in every locale and at every width: a command that does not fit
   scrolls sideways under the kit's thin bar, which floats and takes no height,
   and the icon stays in place. The command is never wrapped or shortened. A
   command that fits has no bar. The notes under a box wrap as ordinary text.

   The card is never cut off (owner request, 2026-11-04), by the rule of every
   screen (Decision 23). While it fits it is centered both ways, looks as it
   always did and nothing scrolls. Below 592 px the area scrolls sideways, and
   the card keeps its 460 px. The longest command needs 372.5 px, so no command
   box scrolls on its own on today's cards. A card taller than the area starts at its top and
   the area scrolls down, so every button can be reached. The bars are the kit's
   thin ones, which float and take no room, and show only while the card
   overflows that way. Dragging a panel edge scrolls nothing. Over a command box
   the plain wheel scrolls the area; a sideways wheel moves the command if it
   does not fit, and at its end the next wheel moves the area. Scrolling stays
   where it is through the search behind the card (below) while the diagnosis
   stays the same. Another diagnosis, and Check again, which shows "searching" and
   so replaces the card, start again at the top.

   Under every card a line lists what OTTY looked for (`$DOCKER_HOST`, the
   sockets, `docker` and `podman` on `PATH`). Links open in the default
   browser with `xdg-open` (Linux) or `open` (macOS) through
   `std::process::Command`; no new crate. Copy uses the iced clipboard.

   The only engine stopping leads to the same card (owner decision,
   2026-11-04): the engine stops answering, OTTY searches again, and when that
   search finds nothing the card of its case replaces the table and the tree
   has no rows. An open Run form waits as typed behind the card and comes back
   with an engine; its row menu and its notice go away. The engine comes back
   by Check again, by entering Docker mode again, or by itself (next paragraph).

   While the card is on screen in active Docker mode, OTTY runs the engine
   search again every 5 seconds behind the card (owner decision, 2026-11-04). The
   card stays and its buttons work: a repeat with the same answer changes
   nothing, another answer changes the card, and an engine takes over as after
   any search. A tick is skipped while such a search is on its way, so a slow one
   cannot stack, and the timer stops as soon as the card goes or the mode is left.
   Check again still shows "searching" until its own search answers; a repeat
   that finishes meanwhile is dropped, so the answer is from after the press.
10. **Resources and columns.** The left tree under each engine root has
    Containers (with "N running"), Compose projects (one child per
    `com.docker.compose.project` label, which filters the containers table),
    Images, Volumes, Networks, each with a count. The table columns:

    | Resource | Columns |
    |---|---|
    | Containers | Name, Status (badge), Image, Ports, Age |
    | Images | Repository, Tag, ID (12 chars), Size, Used by (containers), Age |
    | Volumes | Name, Driver, Used by, Age |
    | Networks | Name, Driver, Scope, Subnet, Containers, Age |

    **Image rows** (owner decision, 2026-11-03). The Images table has one row
    per `repository:tag`, as the `REPOSITORY` and `TAG` columns of `docker
    images` print them. The rows of one image share its ID, size and "Used by".
    An image with no name is one row `<none>` `<none>`. An image known only by
    its digest is one row per repository, with the tag `<none>`. A digest in a
    repository that has a tag gets no row. A row is identified by the image id
    and its name, so selection, the row menu, sorting and a reload follow that
    row. The count of the Images tree row is the number of rows. The status bar
    counts rows too and adds up the size of each image once. Inspect goes by
    image id; the inspector title names the row. A link from a container
    selects the row of the name it was run from (a bare name means `latest`),
    else the first row of its image.

    Above the table: the resource title, the engine chip, a name filter field,
    the resource's create button (Run container, Pull image, Create volume,
    Create network) and Refresh. One row can be selected; the inspector
    shows it. Status badges use the status tones: running → ok, paused and
    restarting → warn, exited with a non-zero code and dead → error, created
    and exited (0) → neutral.

    Engine roots work like folders (owner decision, 2026-11-03), so with
    Docker and Podman on, both can stay open and the user never switches
    between them:
    - Every engine is open by default, also one found later.
    - Pressing a root only opens or closes that engine, on its own. The table
      on the right, the Run form and the inspector do not change, and nothing
      is selected or loaded. To change the table, press a list row inside an
      engine.
    - Closing the engine whose table is shown leaves that table on screen. No
      tree row is highlighted while that engine is closed.
    - Anything else that goes to a list of a closed engine opens that engine,
      so the shown list is visible: a link in the inspector or the table, a
      finished Run, or a search that moves the place from an engine that is
      gone to another engine that is still there. A port link goes to no list
      and opens nothing.
    - An engine the user closed stays closed when it comes back while the
      no-engine card shows (owner decision, 2026-11-04). The table on the right
      shows its containers and its tree row stays closed, so no row is
      highlighted until the user opens it.
    - A closed engine still loads its lists and follows its events; closing
      only hides the rows.
    - Which engines are closed is kept in memory: it survives a mode switch, a
      Refresh and an engine going away and coming back, and it resets when OTTY
      restarts. It is not saved. The one thing that clears it for an engine
      that comes back is a press, a link or a Run that goes to one of its lists.
11. **Actions.**

    | Resource | Actions |
    |---|---|
    | Container | Run (form, Decision 18), Start, Stop, Restart, Pause / Unpause, Remove (confirm; "Also stop it" when running), Logs, Shell; Prune stopped containers (confirm) |
    | Image | Run (opens the Run form with this image), Pull (dialog: name and tag, progress line), Remove (confirm; takes away only this row's name; see "Removing an image row" below); Prune dangling images (confirm) |
    | Volume | Create (dialog: name, driver `local`), Remove (confirm); Prune unused volumes (confirm; anonymous volumes only, with an "Also remove named volumes" switch, off by default, like `docker volume prune` and `-a`; owner approved 2026-11-01) |
    | Network | Create (dialog: name, driver `bridge`), Remove (confirm); Prune unused networks (confirm) |

    Actions on the selected row are buttons in the inspector header and items
    of the row's context menu. Remove is disabled, with a hint under the
    buttons, when the engine would refuse it: a volume or network that a
    container uses, and the built-in networks `bridge`, `host`, `none`. Every destructive action asks first with the
    kit dialog. A failed action shows a kit `Notice` above the table with the
    engine's message; the table stays as it was.

    **Removing an image row** (owner decision, 2026-11-03; read from real runs
    of Docker 29 with its containerd image store and Podman 5, 2026-11-03). A
    row with a tag goes by its reference. A row with no tag goes by image id,
    unless the image has other names: then a digest row goes by its digest.
    "Running" also means paused and restarting; "stopped" also means created.

    | The row | Who uses the image | Without Force | Force switch | With Force |
    |---|---|---|---|---|
    | Not the image's last name | anyone, on either engine | only this name goes | none | not sent |
    | Last name | nobody, on either engine | the image goes | none | not sent |
    | Last name, tagged, Docker | stopped or running containers | refused | on | only the tag goes; the image stays as `<none>` and the containers stay |
    | Last name, no tag, Docker | stopped containers | refused | on | the image goes; the containers stay and can still start |
    | Last name, no tag, Docker | a running container | refused | none, it cannot help | not sent |
    | Last name, Podman | stopped containers | refused | off | the image goes **and so do those containers** |
    | Last name, Podman | a running container | refused | none: it would delete the stopped containers and then still fail | not sent |

    The switch column gives its first state. It starts off on Podman (owner
    decision, 2026-11-03) because Force there also deletes the stopped
    containers, and one confirm must not do that; Docker's Force deletes no
    container, so it starts on.

    The dialog says what goes for the switch as it stands. Where the engine
    will refuse, it says so, and the refusal still reaches the notice.
12. **Inspector tabs.** Overview (key-value list: image, ports, mounts,
    networks, compose project, created, command for containers; matching
    fields for the other resources), Logs (containers only: last 500 lines,
    then follows new lines while the tab is open; mono, scrolls, stops when
    the tab or the selection changes) and Inspect (the engine's inspect JSON,
    read-only, mono).

    The Logs tab has a **Follow** toggle (owner approved, 2026-11-01). On:
    new lines are added and the view stays at the bottom. Off: the view stays
    where it is, new lines wait and a counter says how many; turning Follow
    on adds them and jumps to the bottom. Scrolling up turns Follow off, and
    scrolling back to the bottom turns it on, where iced makes the scroll
    position visible to the app.

    When a container whose logs are open comes back (owner decision,
    2026-11-03), the log view is cleared once and shows only the lines of the
    new run: no separator and no old lines. It does not matter how it came
    back: OTTY's Restart or Start, `docker restart` or stop and start in a
    terminal, or a restart policy. The signal is the engine's `start` event of
    that container, which comes once per run on Docker and on Podman. The
    container list cannot tell, because it can say running before and after a
    fast restart; the end of the log stream cannot tell either, because
    Podman's journald stream goes on across a restart. Engines keep the lines
    of earlier runs, so the new stream reads lines with their times and drops
    those older than the middle of the gap between the end of the last run
    (`FinishedAt`) and the new start (`StartedAt`). Follow keeps its state. A
    container that stays stopped keeps its lines and nothing asks the engine
    again. Opening the tab or selecting the container anew starts with the
    last 500 lines as usual, and those can include earlier runs.

    A `start` event is missed while the engine's events stream is broken, so
    when that stream opens again (owner decision, 2026-11-04) OTTY asks inspect
    about the container whose logs are open on that engine and compares its
    `StartedAt` with the run the stream read when it opened:

    - **Another `StartedAt`:** it was restarted in the gap, so the view is
      cleared and shows only the new run, as for a `start` event. This holds
      also when that run has ended by now.
    - **The same `StartedAt`, running:** the lines stay, waiting ones too, and
      a new stream adds the lines of the gap. Line times alone cannot place
      the new stream: `since` takes whole seconds, and Podman's `k8s-file`
      gives one exact time to all lines written at once. So OTTY goes by the
      time of the last line received and how many lines came with that time.
      The old stream may still be alive, because only the events stream may
      have broken: it is dropped, and what it had not delivered comes through
      the new stream, so no line shows twice or goes missing. At most the last
      5000 lines of the gap are read, which is all the view keeps.
    - **The same `StartedAt`, stopped (owner decision, 2026-11-04):** the lines
      stay and no stream starts. The message of the broken stream goes, because
      the container simply ended, and a late failure of that stream does not
      bring it back. Lines written between the break and the stop are not added;
      opening the tab again reads them.
    - **No answer (removed, or the engine went away again):** the lines stay and
      no stream starts. The message of the broken stream stays, because nothing
      is known about the container.

    Each reconnect asks once and starts at most one stream; nothing else asks
    again. Follow keeps its meaning: with Follow off the new lines wait with
    the counter. An answer that comes after the user moved to another row, tab
    or mode, or after a `start` event started another stream, is ignored. Other
    engines' reconnects and reconnects with no logs open only reload the lists.

    When the only engine is gone for good (Decision 9), the card replaces the
    table and the logs stop. When it comes back (owner decision, 2026-11-04) the
    page and the row the user had come back with it, for any list: the page at
    once, the row when its list loads. A row gone meanwhile falls back to the
    first row, like any missing selection. The Logs tab stays open and starts
    with that row's last 500 lines. Opening another page before the lists load
    wins, and this lives in memory only. When another engine answers first, or
    two engines exist and the shown one goes away, the view moves to the other
    engine and stays there when the first comes back; that engine's lists load
    anew, with their first rows. An engine that the search still finds keeps its
    logs and goes through the reconnect above.
13. **Shell opens a terminal session.** "Shell" opens a new terminal tab that
    runs `docker --host <address> exec -it -- <id> sh` (or
    `podman --url <address> exec -it -- <id> sh` for a Podman engine) and
    switches to Terminal mode. `<address>` is the chosen engine's
    (`unix:///path` or `tcp://host:port`): without it the CLI goes to its own
    default engine, which may be another one (Colima, Docker Desktop, rootful
    Podman). `--` keeps an id from being read as a flag. The tab is grouped
    under "this computer".
    If the matching CLI is not on `PATH`, the button is disabled with a hint.
    No in-app exec over the API: the terminal widget works with a PTY, and the
    CLI already handles TTY sizing and signals.
14. **The tables stay fresh through engine events.** While Docker mode is
    active, a subscription listens to `/events` of each engine and reloads the
    affected list (container, image, volume or network events), at most once
    per 500 ms per list. Entering Docker mode and Refresh reload all lists.
    Nothing polls while another mode is active.
15. **Kit first** (owner rule, 2026-11-01). Every new interface element is a
    component in `otty-ui-kit` with a gallery section in both themes and an
    `iced_test` interaction test before the app uses it:

    | Component | Shape (screen reference) |
    |---|---|
    | `ModeSwitcher` | Raised group of 30×26 icon tabs, active tab `surface-hover` with accent icon, temporary tab with outline, trailing "⋯" button that shows it is open (`.mode-tabs`, `.mode-tab`, `.mode-more`) |
    | `ModeMenu` | Floating menu, 300 wide: section labels with a right-aligned note, rows with icon, name, shortcut `Kbd` and a pin toggle (on: accent; disabled with hint) (`.mm-item`, `.mm-pin`) |
    | `DataTable` | Header row, body rows with one selected row, cell kinds plain / mono / muted / number / badge, empty-table message, vertical scroll, row click and right click messages (`ot-table`) |
    | `StatusBadge` | Dot plus text in tone ok / warn / error / neutral (`ot-status`) |
    | `KeyValueList` | Two-column list, keys muted, values mono, long values wrap (`ot-insp-kv`) |
    | `InspectorHeader` | Mono title, status badge line, wrapping row of small secondary buttons |
    | `InspectorTabs` | Underlined text tabs (`ot-insp-tabs`) |
    | `LogView` | Mono, selectable, scrolls, sticks to the bottom while following |
    | `Notice` | Inline message with tone, text and dismiss button (`ot-banner`) |

    Existing parts are reused: `TreeRow` (gains a count tone, e.g. failed),
    `EmptyCard`, `Chip`, `TargetDot`, `Dialog`, `Menu`, `TextField`,
    `TokenField`, `KeyValueField` (gains a placeholder per column if it has
    none), `Select`, `Segmented`, `Switch`, `FormPage`, `FormSection`,
    `Button`. App views only compose kit components and pass `UiPalette`.
16. **New dependencies.** `bollard = "0.21.1"` (default features: unix socket
    and http; no ssl, no ssh) and `futures-util` (already in the tree through
    bollard and iced) to read bollard's streams, both only in `otty-docker`
    (owner approved, 2026-11-01). `cargo deny check` must stay clean.
17. **All new text is translated** in the ten locales of `app/src/i18n/locales`.
18. **Run container is a form in the main area** (owner request, 2026-11-01),
    laid out like the launch editor. It opens from "Run container" above the
    Containers table (empty image field) and from "Run" on an image (image
    filled in).

    | Section | Fields |
    |---|---|
    | Image | Engine (select; shown only when more than one engine is reached), Image (`name:tag`, required), Name (optional; the engine picks one when empty), "Pull the image if it is missing" (switch, on) |
    | Command | Command (tokens, optional; replaces the image's command) |
    | Ports | Rows of host port → container port, protocol `tcp` / `udp` |
    | Environment | Rows of name = value |
    | Volumes | Rows of source (volume name or absolute host path) → container path, read-only switch |
    | Options | Network (select of the engine's networks, default `bridge`), Restart (segmented: No, On failure, Unless stopped, Always), "Remove the container when it stops" (switch, off) |

    "Remove the container when it stops" and a restart policy cannot go
    together (the engine rejects it), so turning the switch on sets Restart
    to No and disables the other choices with a hint. The right panel shows
    what the chosen image sets on its own (listening ports, command,
    entrypoint, workdir), or that it is not on this computer yet.

    Checks before Run: image is not empty; ports are 1–65535; container
    paths are absolute; a host path source exists. Each problem shows under
    its field and Run stays disabled. On Run: pull the image if it is missing
    and the switch is on (progress line in the footer), create the container,
    start it, then show the Containers table with the new container selected.
    An engine error shows a `Notice` at the top of the form and keeps every
    field as typed. Containers that need a terminal (`-it`) are started
    detached; the user opens them with Shell.

    A typed folder is asked about on disk after a 300 ms pause in typing, and
    Run stays disabled until the disk answers. With no answer after 3 seconds
    the row says the folder is not answering (a network drive may be
    unavailable), and Run stays disabled: it is never enabled after a timeout.

    A source of `~` or `~/…` stands for the home folder, read during the engine
    search. When the computer does not say where the home folder is, the row
    asks for the full path and Run stays disabled: such a source is never read
    as a folder on the root or as a volume name.

19. **Owner review, round 1** (2026-11-01). Where this list and an earlier
    decision or a screen disagree, this list wins.
    - **No Compose projects in the tree.** The tree under an engine root is
      Containers, Images, Volumes, Networks. The Containers table gets a
      **Compose** column; the project name in a row is a link, and pressing
      it filters the table to that project (the "compose: <name>" chip with
      its clear button stays above the table).
    - **Sorting.** Every table can be sorted by pressing a column header;
      pressing again reverses it; an arrow shows the column and direction.
      The order follows the column's meaning: text alphabetically
      (case-insensitive), ages and dates by time (first press: newest first),
      sizes and counts by number (first press: largest first), status by
      state (running, restarting, paused, created, exited, dead). Default
      order: containers running first, then by name; images, volumes and
      networks by name. Rows with an equal value follow the name, also ignoring
      case. Two texts that differ only in case keep a fixed order, so rows do
      not jump between rebuilds. The chosen sort is kept per page (Decision 21).
    - **No "this computer"** in Docker mode: engine roots in the tree show
      "Docker 29.9.1" only, and the header above the table has no engine
      chip (the version is already in the tree and the status bar).
    - **Ports read like `docker ps`:** `8080:80/tcp` (host:container), in
      the table, the inspector and the Run form's row separator.
    - **Header layout:** first row the title (and the compose chip when a
      project filter is on); second row the controls: the name filter, then
      the actions as icon buttons with tooltips (create / run, prune,
      refresh); then the table. When the controls row is too narrow, the
      action buttons move into a "⋯" menu at the end of the row; the filter
      stays and shrinks first.
    - **Row "⋯" button:** every table row whose resource has actions ends
      with a "⋯" button. It opens the same actions menu as a right click,
      for that row, without changing the selection or opening the
      inspector.
    - **Inspector actions as icon buttons** with tooltips (Remove keeps its
      danger color); the hint under them stays.
    - Icons are original simple stroke icons in the style of
      `assets/svg/*` (24×24 view box, round caps, one stroke width); no
      third-party icon set is copied.

20. **Owner review, round 2** (2026-11-02). Wins over earlier decisions and
    screens where they disagree.
    - **Run form, SELinux:** on a system where SELinux is enabled
      (`/sys/fs/selinux/enforce` exists), a volume row whose source is a
      folder on this computer has a switch "Let the container read this
      folder (:z)", off by default, with a hint that it changes the SELinux
      labels of the folder's files. On, the bind gets the `z` option. Other
      systems never show it.
    - **Run form, Restart** starts at No (owner approved).
    - **Pressing a table row opens the right panel** when it is closed, so
      the inspector shows the row. The row "⋯" button and the right click do
      not open it.
    - **Inspector layout:** the same side padding for every part; the status
      line and other text wrap instead of being cut; values wrap at spaces,
      slashes and dashes, not in the middle of a word; dates read
      `2026-11-01 18:30 UTC`. When the tabs do not fit the panel width,
      they become a kit select with the same choices.
    - **No mode name** next to the mode switcher; the active tab is enough.
    - **Links in Overview:** a published TCP port opens
      `http://localhost:<host port>` in the browser; the image, network and
      volume values jump to that list and select the item; the compose
      project opens the compose filter. Values that point nowhere stay text.
    - **Inspect is a readable tree, not raw JSON:** collapsible sections for
      the top-level keys (State, Config, HostConfig, NetworkSettings,
      Mounts, …), nested objects as indented key and value rows, lists as
      numbered rows, empty values hidden behind "Show empty"; a "Raw JSON"
      switch shows the JSON as today, and an icon button copies it. Values
      in the tree that name something OTTY can show are links, like in
      Overview (owner request, 2026-11-02).
    - **Polish from round 1:** the row "⋯" menu opens at its button; the sort
      arrow sits right after the column title.
    - **Port links in the table** too: each published TCP port in the
      Containers table's Ports cell links like in the Overview, without
      selecting the row (owner request, 2026-11-02).
    - **Narrow inspector lists stack:** when the key-value list is narrow,
      each key sits on its own line with its value under it, instead of two
      columns (owner request, 2026-11-02).

21. **Tag search** (owner request, 2026-11-02). Wins over earlier decisions
    where they disagree. Built to be reused by other widgets: the kit holds
    the model and the views, and a widget only supplies its keys and values.
    - **Layout:** title row; then the search field and action buttons; then
      a row of tags; then the table. The tags row wraps and is not shown when
      there are no tags. The compose chip next to the title goes away.
    - **Plain text** in the field filters by name, as today.
    - **Tags are `key:value`.** The key is a column of the table on screen,
      written as its header in the interface language: you write what you see
      (owner request, 2026-11-03). `Имя:web-1` in Russian, `Name:web-1` in
      English. Case does not matter; a header with a `:` gives the part before
      it; a wide `：` counts as `:`. Each
      key also has a stable internal name (`name`, `status`, `image`,
      `compose`, `ports`, `age`, `repository`, `image-tag`, `id`, `size`,
      `used-by`, `driver`, `scope`, `subnet`, `containers`) that is never typed
      or shown.
      Text before the first colon counts as a key only if it is the shown name
      of a column of this table; otherwise the whole text is a name filter (so
      `redis:7` stays plain text).
    - **Matching:** case-insensitive substring on the text shown in the cell;
      a cell with several values (ports) matches if any does. Different keys
      must all match; several tags with one key match if any does.
    - **Suggestions while typing:**
      - no key yet: the matching keys in column order, then matching names;
      - after `key:`: the values of that column, from the rows the other
        filters leave.
      Above them, the first row is the typed text itself (not for an empty
      field or a bare `key:`, and only once if it equals a suggestion). It is
      highlighted first (owner report, 2026-11-03). ↑ ↓ move, Esc closes, a
      click picks. An empty focused field shows the keys.
    - **Enter applies the highlighted row:** the typed text stays in the field
      as a name filter, or `key:text` becomes a tag that matches by substring;
      a name fills the field and filters; a key puts `key:` in the field and
      shows its values; a value adds the tag and clears the field. **Tab
      completes to the first real suggestion**, like a shell, so `sta` Tab,
      `run` Tab gives the tag `Status:Running`. Backspace in an empty field
      removes the last tag.
    - **Removing:** every tag has a clear cross, and the tags row ends with a
      "Clear all" button that removes all tags of this table.
      A long tag wraps and its chip grows in height; it is never cut (owner
      report, 2026-11-03).
    - **Compose is a tag:** the Compose link adds `compose:<name>`, replacing
      another `compose` tag.
    - **One search and one sort per page** (owner decision, 2026-11-04). A page
      is one list of one engine: the containers of Docker, the images of Podman.
      Each page has its own tags, its own typed text and its own sort column
      and direction; nothing on one page filters or orders another. The
      suggestions come from that page's rows, and positions in a filtered table
      are read on the page that drew it.
      - A page keeps its tags, text and sort when the user goes to another page
        and back (the text since owner decision, 2026-11-04). The suggestion list
        closes when the page changes and when the user leaves Docker mode; the
        text stays and filters as before.
      - A page keeps its tags, text and sort when its engine goes away and comes
        back, so a brief outage does not wipe a filter. The list closes then too.
      - All of this lives in memory and resets when OTTY restarts. Not saved to
        disk.
      - A link to an item clears the text of the page it lands on and removes
        the tags that would hide the item, so the item is shown. The Compose
        link clears that text too, and adds its tag on the Containers page of
        the engine it was pressed on, and only there. The page a link leaves
        keeps its text.
22. **The mode is called Containers** (owner request, 2026-11-04): the dock, the All modes menu, the palette and the status bar say Containers where they name the mode, translated in every locale; "Docker" and "Podman" still name the engines (tree roots, cards, status bar), and `Mode::Docker`, `widgets/docker` and the intents keep their names.
23. **A screen keeps a minimum width and scrolls** (owner decisions, 2026-11-04). One rule, set in one place, so a new screen gets it without knowing about it. Everything that fills the main area goes through one kit component, `ScreenArea`, chosen in `app/src/view.rs` (`MainContent`): Settings, the launch editor, the error tab, the start card, the Run form, the searching card, each no-engine card, and the line a tab shows when its state is missing. What is inside does not matter.

    - **The rule.** With room for it a screen looks as it always did: it takes the area's width, or is centered if it is smaller, like a card. Below the minimum it keeps that width and the area scrolls sideways. A screen lower than the area gets the area's height, so a form's footer lands at the bottom and a card is centered; a taller one starts at its top and the area scrolls down. There is one scroller and the bars are the kit's thin ones, which float and take no room.
    - **One minimum.** 592 px for every screen (`size::SCREEN_MIN`), side padding included, accepted by the owner in place of one number per screen. The Run form sets it, as the widest: 520 px of content and 36 px of padding on each side, sized for the longest locale (Russian and German; Korean is not counted, since Hangul falls back to a wider font for now). So Settings and the launch editor start scrolling earlier than they used to (552 and 472 px), and the cards too: a card no longer narrows below 460 px, and in a narrower area it stays centered in the 592 px. The read-only column of a volume row is 140 px, measured in the kit's fonts: the longest label is Portuguese at 139.4 px, then Russian at 132.8 and French at 123.5, and Korean is left out for the same reason. It leaves the source and target inputs 156 px each at the minimum, so 592 px still holds.
    - **Exceptions.** The panes of a terminal, and table screens with the toolbar above them: a table's sticky header and its own row scrolling need a height they can fill, and in an outer scroller the height has no end.
    - **Not in this container.** Dialogs lie above the screen and have one scroller for their card, which keeps its width in a window too small for it. Side panels wrap their text (ticket 41). A command block keeps its own sideways scroll.
    - **Each screen has its own scrolling.** Each screen has a name (`ScreenId`): one per tab, the start card, the Run form, the searching card and one per no-engine diagnosis. Another name starts at the top left, with no field focused. The same name built again keeps its place: typing, a validation message, the search every 5 seconds. Coming back to a screen after another one starts at the top too; the area keeps no memory of screens, and what was typed stays because the app holds it.

    **One scroll, one footer, the buttons at the bottom** (owner request, 2026-11-04, replaces the pinned Run bar with its own sideways scroll). A form and its buttons are one screen: each form page has one scroller, both ways, holding the fields and the buttons, and nothing else on the page scrolls them. All form pages place their buttons the same way, through the one footer slot of `FormPage`: a bar at least 60 high with a line on top. The line spans the page, and what the bar holds sits in the same centered column as the fields, at most `size::FORM` (720 px) wide with `space-7` side padding, so the buttons line up with the right edge of the form and a hint or pull line starts at its left edge (owner request, 2026-11-04). Up to 792 px the form fills the page, so there the bar's content spans the page as before. The buttons sit at the right with the main action last. It holds Reset and Save in Settings, Cancel and Save in the launch editor, and the pull line or hint, Cancel and Run in Run container, whose fields use the same column. Reset has the look of Cancel (Ghost: no box at rest, a box under the pointer); its label, state and action did not change. While a form is lower than the area, the bar sits at the bottom of the area. When the form is taller, the bar comes after the fields and scrolls with them, so on a tall form the user scrolls down to reach Run; the owner accepted this. Below the minimum width the fields and the bar scroll sideways together. A hint or a pull line wraps beside the buttons and the bar grows with it; the buttons always keep their size. Text fields, command blocks and the list of a select still scroll their own long content.

## Ticket classes

Same classes as `specs/2026-10-31-workspace-layout`: **A** restyle / kit,
**B** new view of existing data, **C** behavior (tests first). Each ticket has
a `Class:` line; review checks the diff against it.

## Acceptance Criteria

- [x] AC1: The left panel starts with the switcher: Terminal tab, the pinned tabs, "⋯". Clicking a tab switches the left panel, main area, right panel and status bar to that mode; the terminal tabs keep running and are back as they were when Terminal mode returns.
- [x] AC2: The All modes menu lists Pinned (with "N of 4") and Other modes; Pin and Unpin change the tabs at once; Terminal has no Unpin; with 4 pinned, Pin is disabled with the hint; picking an unpinned mode shows it as a temporary outlined tab that goes away when another mode is chosen.
- [x] AC3: Pinned modes survive a restart (`modes.json`); a missing or broken file gives the defaults; OTTY starts in Terminal mode.
- [x] AC4: The shortcuts in Decision 6 work with and without a focused terminal; Ctrl 1–5 and Ctrl M without Shift still reach the shell (keymap unit test plus a manual `cat -v` check). The palette switches modes.
- [x] AC5: With Docker running, Docker mode shows a "Docker <version>" root with Containers, Compose projects, Images, Volumes, Networks and their counts; with Podman (`podman.socket` on) a "Podman <version>" root; with both, two roots. Lists match `docker ps -a`, `docker images`, `docker volume ls`, `docker network ls` (and the `podman` equivalents); the Images table and the count of its tree row have one row per `repository:tag`, as the `REPOSITORY` and `TAG` columns of `docker images` print them (Decision 10; `the_image_rows_match_the_command_line` in `crates/docker/tests/live_engine.rs` compares them on a real engine).
- [x] AC6: Each case in Decision 9 shows its text and buttons; the install and docs buttons open the browser; Copy command puts the command on the clipboard, on each command of a card with two; Check again finds an engine started meanwhile without a restart, and OTTY finds it by itself within about 5 seconds while the card shows, with no flicker of the card.
- [x] AC7: Every action in Decision 11 works on Docker and on Podman; destructive actions ask first; an engine error shows a Notice and leaves the table as it was (reducer tests for each action's effect; mapping tests in `otty-docker`).
- [x] AC8: The inspector shows Overview, Logs (follows new lines, stops on tab or selection change) and Inspect for the selected row.
- [x] AC8a: Run container starts a container with the image, name, command, ports, environment, volumes, network, restart policy and auto remove from the form (checked with `docker inspect` / `podman inspect`); a missing image is pulled first when the switch is on; each check in Decision 18 blocks Run with its message; an engine error keeps the form as typed (reducer tests for the checks and for the mapping of the form to the create request).
- [x] AC9: Shell opens a terminal tab inside the container with the engine's own CLI and switches to Terminal mode; without the CLI the button is disabled with a hint.
- [x] AC10: Starting, stopping or removing a container from another terminal updates the table within a second without Refresh; nothing talks to the engine while Terminal mode is active (event subscription test on the mode).
- [x] AC11: Every component in Decision 15 is in `otty-ui-kit`, shown in `cargo run -p otty-ui-kit --example gallery` in both themes, with at least one `iced_test` interaction test.
- [ ] AC12: Screenshots of Terminal mode with the switcher, the open All modes menu, each Docker table, the inspector tabs and each Decision 9 case, in OTTY Dark and OTTY Light, are attached to the PR.
- [x] AC13: `cargo +nightly fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features` and `cargo deny check` pass; line coverage from `cargo llvm-cov --workspace --all-features` is strictly greater than on the merge base.

## Out of Scope

Shown on the screens or easy to imagine, but each needs its own spec.

- Plugins and extensions: "Get more modes from extensions…", the "+ Add a mode" rail button, plugin tabs in the right panel (Git, Notes), plugin items in the status bar (AWS, branch).
- Kubernetes, agent work, PostgreSQL, AWS and other modes.
- The bottom terminal dock inside Docker mode (`V4FutureModes`, `V4ModesMany`).
- Remote engines (SSH, tcp with TLS), Docker contexts, Docker Swarm.
- CPU and memory columns (they need a stats call per container).
- Running a container attached to a terminal (`run -it`), resource limits, health checks and labels in the Run form; building images, `docker compose up/down`, connecting containers to networks, editing containers.
- Search, drag to reorder and the grip in the All modes menu.
- Multi-select and column sorting in tables.
- Windows (not a supported platform).

## Tickets

| # | Ticket | Class | Model | Blocked by |
|---|---|---|---|---|
| 01 | Kit: `ModeSwitcher`, `ModeMenu` | A | Sonnet | — |
| 02 | Kit: `DataTable`, `StatusBadge`, `Notice`; `TreeRow` count tone | A | Opus | — |
| 03 | Kit: `KeyValueList`, `InspectorHeader`, `InspectorTabs`, `LogView` | A | Sonnet | — |
| 04 | `modes` widget: `Mode`, active mode, pin rules, menu state, `modes.json` | C | Opus | — |
| 05 | Mode shortcuts and palette actions | C | Opus | 04 |
| 06 | Root view by mode: switcher and menu in the left panel; Docker slots show a placeholder | C | Opus | 01, 04 |
| 07 | `otty-docker` crate: add bollard (owner approval), engine discovery for Docker and Podman on Linux and macOS, `EngineInfo` | C | Opus | — |
| 08 | `otty-docker`: list and map containers, images, volumes, networks | C | Opus | 07 |
| 09 | `docker` widget: engines, tree, tables, selection, filter, Refresh | C | Opus | 02, 06, 08 |
| 10 | No engine states (Decision 9), open links, copy command | C | Sonnet | 06, 07 |
| 11 | Container actions, confirm dialogs, notices | C | Opus | 09 |
| 12 | Image pull, volume and network create, remove and prune | C | Opus | 09 |
| 13 | Inspector: Overview, Logs (follow), Inspect | C | Opus | 03, 09 |
| 14 | Run container form (Decision 18) | C | Opus | 09, 12 |
| 15 | Shell into a container as a terminal tab | C | Sonnet | 11 |
| 16 | Live updates from engine events | C | Opus | 09 |
| 17 | Compose projects in the tree | B | Sonnet | 09 |
| 18 | Status bar per mode | B | Sonnet | 09 |
| 19 | OTTY Light pass and screenshots | A | Sonnet | 10–18 |
| 20 | Update the design system and the canvas boards to what was built (new kit parts, shortcut table on the old mode boards) | — | Opus, owner review | 19 |
| 21 | Discovery: more sockets, CLI install folders, `podman-docker` wrapper (Decision 8 additions) | C | Opus | 07 |
| 22 | Logs Follow toggle (Decision 12) | C | Opus | 11, 13 |
| 23 | Volume prune: anonymous only by default, switch for named volumes (Decision 11) | C | Sonnet | 12 |
| 24 | Kit: sortable `DataTable` headers, clickable link cell, row "⋯" button column; collapsing `Toolbar`; icon buttons with tooltips (Decision 19) | A | Opus | 02 |
| 25 | Docker tables: Compose column with link filter, no Compose tree group, no "this computer" / engine chip, `host:container` ports (Decision 19) | B | Sonnet | 24 |
| 26 | Docker tables: sorting, defaults and kept choice (Decision 19) | C | Sonnet | 24, 25 |
| 27 | Docker header layout with collapsing controls, row "⋯" menus, icon actions in the inspector (Decision 19) | C | Opus | 24, 26 |
| 28 | Polish: no mode name, row ⋯ menu at its button, sort arrow after the title, inspector wrapping, padding and short dates (Decision 20) | A | Sonnet | 27 |
| 29 | Run form: SELinux `:z` switch, Restart starts at No (Decision 20) | C | Sonnet | 14 |
| 30 | Row press opens the right panel; inspector tabs become a select when narrow (Decision 20) | C | Sonnet | 28 |
| 31 | Overview links: ports to localhost, image/network/volume jump, compose filter (Decision 20) | C | Opus | 28 |
| 32 | Inspect as a readable tree with Raw JSON and Copy (Decision 20) | C | Opus | 28 |
| 33 | Bug: opening or activating a tab from another mode switches to Terminal mode | C | Sonnet | 31 |
| 34 | Port links in the Containers table (Decision 20) | C | Sonnet | 31, 33 |
| 35 | Key-value list stacks key over value when narrow (Decision 20) | A | Sonnet | 34 |
| 36 | Bug: narrow window keeps tree labels and short table columns readable (Decision 19) | A | Sonnet | 19 |
| 37 | Inspect: Copy JSON as an icon, links in the tree (Decision 20) | C | Opus | 32 |
| 38 | Bug: the volume prune dialog says what will really be removed (Decision 11) | C | Sonnet | 37 |
| 39 | Bug: Inspect controls: labels on one line, copy icon never overlaps | A | Opus | 37 |
| 40 | Bug: dragging a split never scrolls, dragging a scrollbar never resizes | C | Opus | 39 |
| 41 | Bug: text in narrow panels stays on one line, nothing overlaps | A | Sonnet | 40 |
| 42 | Narrow Inspect: the copy icon sits on the left under the switches | A | Sonnet | 41 |
| 43 | Kit: tag search model (parsing, suggestions, matching) (Decision 21) | C | Opus | 42 |
| 44 | Kit: `TagSearchField` and `TagRow` views (Decision 21) | A | Opus | 43 |
| 45 | Docker tables use tag search; compose becomes a tag; tags kept per page (per table until 2026-11-04) (Decision 21) | C | Opus | 44 |
| 46 | Fuzzy suggestions with `frizbee` (dependency approved by the owner, 2026-11-02) | C | Sonnet | 45 |
| 47 | Bug: after a suggestion is applied the cursor goes to the end of the field | C | Sonnet | 46 |
| 48 | Bug: the text you typed is the first suggestion (Decision 21) | C | Sonnet | 47 |
| 49 | Search keys are written in the interface language (Decision 21) | C | Opus | 48 |
| 50 | Bug: a long tag wraps and its chip grows (Decision 21) | A | Sonnet | 49 |
| 51 | Bug: icon buttons light up on the whole button; danger has a visible hover | A | Sonnet | 50 |

Order: (01, 02, 03, 04, 07 in parallel) → (05, 06, 08) → (09, 10) → (11, 12, 13, 16, 17, 18) → (14, 15) → 19 → 20. App tickets that touch `app/src/view.rs` run one at a time. One subagent and one commit per ticket; each commit leaves the app working.

Tickets live in `issues/`. Each ticket names the model that runs it on its `Model:` line; a subagent started for a ticket is launched with that model.

## Progress Tracking

- [x] 01 — Kit: ModeSwitcher, ModeMenu · Sonnet
- [x] 02 — Kit: DataTable, StatusBadge, Notice · Opus
- [x] 03 — Kit: KeyValueList, InspectorHeader, InspectorTabs, LogView · Sonnet
- [x] 04 — `modes` widget · Opus
- [x] 05 — Mode shortcuts and palette · Opus
- [x] 06 — Root view by mode · Opus
- [x] 07 — `otty-docker`: engine discovery · Opus
- [x] 08 — `otty-docker`: lists · Opus
- [x] 09 — `docker` widget · Opus
- [x] 10 — No engine states · Sonnet
- [x] 11 — Container actions · Opus
- [x] 12 — Image, volume, network actions · Opus
- [x] 13 — Inspector tabs · Opus
- [x] 14 — Run container form · Opus
- [x] 15 — Shell into a container · Sonnet
- [x] 16 — Live engine events · Opus
- [x] 17 — Compose projects · Sonnet
- [x] 18 — Status bar per mode · Sonnet
- [x] 19 — OTTY Light pass and screenshots · Sonnet
- [x] 20 — Design system and canvas sync · Opus, owner review
- [x] 21 — Discovery additions: sockets, CLI folders, `podman-docker` · Opus
- [x] 22 — Logs Follow toggle · Opus
- [x] 23 — Volume prune keeps named volumes by default · Sonnet
- [x] 24 — Kit: sortable table, link cell, row ⋯, Toolbar, icon tooltips · Opus
- [x] 25 — Compose column, no "this computer", `host:container` ports · Sonnet
- [x] 26 — Table sorting · Sonnet
- [x] 27 — Header layout, row ⋯ menus, icon actions · Opus
- [x] 28 — Polish: mode name, menu position, sort arrow, inspector text · Sonnet
- [x] 29 — Run form: SELinux `:z`, Restart No · Sonnet
- [x] 30 — Row opens the right panel; tabs to select when narrow · Sonnet
- [x] 31 — Overview links · Opus
- [x] 32 — Inspect as a readable tree · Opus
- [x] 33 — Bug: tab open switches to Terminal mode · Sonnet
- [x] 34 — Port links in the Containers table · Sonnet
- [x] 35 — Key-value list stacks when narrow · Sonnet
- [x] 36 — Bug: narrow tree labels and table columns · Sonnet
- [x] 37 — Inspect: copy icon, links in the tree · Opus
- [x] 38 — Bug: volume prune dialog text · Sonnet
- [x] 39 — Bug: Inspect controls layout · Opus
- [x] 40 — Bug: resize never scrolls, scrollbar never resizes · Opus
- [x] 41 — Bug: narrow panel text never squeezes · Sonnet
- [x] 42 — Copy icon under the switches when narrow · Sonnet
- [x] 43 — Kit: tag search model · Opus
- [x] 44 — Kit: tag search views · Opus
- [x] 45 — Docker tables use tag search · Opus
- [x] 46 — Fuzzy suggestions with `frizbee` · Sonnet
- [x] 47 — Bug: search cursor after a suggestion · Sonnet
- [x] 48 — Bug: typed text is the first suggestion · Sonnet
- [x] 49 — Search keys in the interface language · Opus
- [x] 50 — Bug: a long tag wraps · Sonnet
- [x] 51 — Bug: icon button hover · Sonnet
