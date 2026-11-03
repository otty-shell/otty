# 12 — Image pull, volume and network create, remove and prune

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Actions for images, volumes and networks (spec Decision 11). Tests first.
Screens: `DockerImages`, `DockerVolumes`, `DockerNetworks`.

- Pull image dialog: name and tag (default `latest`), a progress line built from the pull stream, Cancel stops it. Progress aggregation over layers is a pure, tested function in `otty-docker`.
- Image remove asks first, with the switch "Force" when a stopped container uses it; Prune dangling images asks first.
- Create volume dialog (name, driver `local`); remove asks first; Prune unused volumes asks first.
- Create network dialog (name, driver `bridge`); remove asks first; Prune unused networks asks first.
- Remove is disabled with a hint for a volume or network a container uses and for `bridge`, `host`, `none`.
- Failures show a `Notice`; success reloads the affected list.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests for every action and dialog; tests for the disabled-Remove rules; pull progress aggregation tests.

## Answer

Images, volumes and networks have their actions, dialogs and prunes, and
the create buttons above those tables work. Tests were written first and
seen failing (missing calls, types, intents and keys), then the code.

### `otty-docker`

- `connection/resource_actions.rs`: `remove_image(id, force)`,
  `prune_images()` (filter `dangling=true`), `create_volume(name, driver)`
  (a blank name lets the engine pick one), `remove_volume(name)`,
  `prune_volumes()`, `create_network(name, driver)`, `remove_network(id)`,
  `prune_networks()`.
- Volume prune removes every unused volume on both engines: Docker 23 and
  later (API 1.42) prune only anonymous volumes unless `all=true`, so
  `volume_prune_options` sends that filter to Docker with API ≥ 1.42 only;
  older Docker and Podman prune every unused volume without it.
- `pull.rs`: `PullProgress` (public, `Default`), added up over the layers
  of the pull stream: `layer_count`, `finished_layers` ("Pull complete",
  "Already exists"), `downloaded`, `total` (bytes of layers with a known
  size), `fraction` (by bytes, else by finished layers). Lines about the
  whole image (`Pulling from`, `Digest:`, `Status:`) are not layers;
  "Verifying Checksum", "Download complete" and "Extracting" count the
  layer as downloaded.
- `connection/image_pull.rs`: `Engine::pull(image, tag)` →
  `BoxStream<'static, Result<PullProgress, EngineError>>`, a snapshot per
  line; an engine error in the stream ("manifest unknown") is an `Err`
  whose `message()` is the engine's text. Dropping the stream stops the
  pull.
- Fixtures: `tests/fixtures/docker-pull.jsonl` (written by hand in the
  shape Docker 29 sends; no image was pulled), `fixtures::pull_progress()`
  (half way: 3 layers, 15 of 47 MB) and `fixtures::idle_resources(kind)`
  (images, volumes and networks as if no container existed, for the
  removable cases).

### Kit

`ProgressBar` (`components/progress_bar.rs`, `.prog`): 160×4, `accent`
fill on a `surface-raised` track, radius 2, `width()`; the fraction is
clamped (NaN is empty). Gallery section "ProgressBar"; `iced_test` layout
tests (size, width) and a contrast test of the fill on dialogs.

### App (`app/src/widgets/docker/`)

- `actions.rs`: `RowAction::PullAgain`; `EngineAction` gained the image,
  volume and network calls (list, target, failure key; the dialog switch
  forces an image removal too); `image_actions` (tagged: Pull again,
  Remove; dangling: Remove), `volume_remove_hint` (a container uses it:
  "Remove the container <names> first; …"), `network_remove_hint`
  (built-in `bridge`/`host`/`none`: "The engine creates this network
  itself …"; containers connected: "Stop or remove the connected
  containers first.").
- `action_flow.rs`: one `SelectedItem` per list (offered actions, whether
  Remove is blocked, the remove switch: "Stop it first" for containers,
  "Force" on by default for an image a container uses, as on the screen).
  A blocked Remove does nothing; Pull again opens the pull dialog with the
  image's name and tag (`forms::split_reference`). Prune works on every
  list and counts what goes (stopped containers, dangling unused images,
  unused volumes, unused user networks). `reload` and `run` are shared
  with the forms.
- `forms.rs`: `Form` (kind, engine, name, tag, driver, progress while
  pulling), `FormKind`, `FormField`; defaults tag `latest`, drivers
  `local` and `bridge`; `can_submit` (pull: image and tag; network: name
  and driver; volume: driver); fields keep still while a pull runs.
- `form_flow.rs`: `OpenCreate` opens the dialog of the open list (none for
  Containers until ticket 14), `FormSubmitted` starts the pull
  (`StartPull`; the dialog stays with its progress) or runs the create and
  closes, `FormCancelled` closes and, while pulling, emits `CancelPull`,
  `PullFinished` closes the dialog and reloads, or shows "Could not pull
  nginx:latest: <message>"; a result without a running pull is dropped.
- `action_model.rs`: buttons and hint for every list (Remove disabled with
  the hint), the remove and prune dialogs of every list, Prune buttons
  ("Prune stopped", "Prune dangling", "Prune unused"), `can_create`.
  `form_model.rs`: `FormViewModel` (fields with labels and placeholders,
  progress line "Pulling <ref>", bar fraction, "15 MB of 47 MB").
- Views: `view/form_dialog.rs` (kit `Dialog`, `TextField`s, `ProgressBar`,
  ghost Cancel, primary Pull or Create); the create button in
  `view/main_area.rs` sends `OpenCreate`; the dialog is a window layer in
  `view.rs` like the others.
- `events/docker.rs`: `StartPull` runs `Task::run` over an
  `iced::stream::channel` (a `PullProgressed` per line, then one
  `PullFinished`), made `abortable`; `DockerWidget` keeps the handle
  (`abort_on_drop`) and `CancelPull` aborts it. `RunAction` runs the new
  calls. `guards.rs`: the pull intents are background events.
- i18n: 38 keys in the ten locales; `Key::ALL` is 328. Helpers
  `docker_volume_in_use`, `docker_pulling`, `docker_pull_amount`.

### Tests

Crate: every call's method, path, query and body against the fake engine
(force only when asked, `dangling`, `all`, JSON `Name`/`Driver`, blank
volume name), the `all` filter rule per engine and API version, an engine
refusal's message, pull aggregation (half way, finished, extracting,
whole-image lines, no sizes, a layer seen again), the pull stream (request
and snapshots, stream error), fixtures. App: image actions, remove hints
(used, unused, built-in, connected), every call's list and target, Force
switch, one failure text per call; reducer: image remove with and without
Force, Pull again and not for a dangling image, blocked volume and network
removals, unused volume and network removal, container actions ignored on
other lists, every prune with its count; forms: defaults, required
fields, reference splitting, pulling locks the fields; form flow: open per
list, typing, pull start, progress, second submit, success, failure,
cancel while pulling and before, dropped late results, create volume and
network, no form; view models and `iced_test` views (fields, Cancel,
disabled action, progress line, create button); root view shows the pull
dialog; events (pull task kept and aborted); guards; i18n templates.

Coverage (lines): 84.34% on the ticket 11 commit → 84.90%.

### Deviations and choices

- "Prune unused volumes" removes named unused volumes too (the label and
  Podman's behaviour); the dialog says so and gives the count.
- "Pull again" (on the screen, not in the ticket) is there; Run on an
  image is ticket 14.
- An image is removed by id; an image with several tags then needs Force
  (the engine says so in the notice).
- A failed pull closes the dialog and shows the notice above the table,
  as the ticket says, so the typed name is lost.
- No live check against a real engine; all calls are tested on the fake
  engine.

### After the rebase onto tickets 13 and 16

Both commits were rebased onto `docker-view` with tickets 13 (inspector
tabs) and 16 (engine events). The inspector now shows the header with
the action buttons, then the tabs (`InspectorProps` has `actions` and
`tabs`); the action and pull results join 13's and 16's background
intents in `guards.rs`. A successful action still emits `LoadLists`
(the reducer tests rely on it, and it keeps the table right when the
events watch is down), so with the watch running the lists may load once
more; that is cheap and left as is. Coverage (lines) on the rebased
commits: `docker-view` (`b48bd2f`) 84.36% → ticket 11 84.82% → ticket 12
85.33%.

### For the next tickets

- 14 (Run): reuse `StartPull`/`PullProgressed`/`PullFinished` and
  `PullLineViewModel` + kit `ProgressBar` for the footer line; enable the
  Containers create button in `action_model::can_create` and
  `form_flow::open`; add Run to `image_actions`.

## Comments

### 2026-11-03: one row per tag; removal rules from real engines

Owner decision: the Images table has one row per `repository:tag` with the tag
in its own column, and Remove takes away only that row's name. This replaces
"An image is removed by id; an image with several tags then needs Force" above,
and "Force when a stopped container uses it" in What. The rules are in spec
Decision 10 (rows) and Decision 11 ("Removing an image row").

The old rule was wrong in both directions. With no containers, an image with
several tags was refused and the dialog offered no way past it. With a
container, Force on by default took every tag while the dialog named one.

Real `DELETE /v1.53/images/{name}` calls on Docker 29.9.1 (containerd image
store) and Podman 5.8.8, on throwaway images:

- Not the last name: only that name goes, whatever containers do, on both.
- Last name and no container: the image goes, on both.
- Last tag, a stopped or running container: refused on both. With Force, Docker
  takes the tag away and keeps the image and the container; Podman deletes the
  stopped container too, and refuses when a container runs.
- Podman, one stopped and one running container, Force: the stopped container
  is deleted and then the call fails. So Podman gets no switch while a container
  runs.
- No tag, by id: stopped is refused and Force deletes the image (Docker keeps the
  container, and it can still start; Podman deletes it); running is refused even
  with Force.
- By id with several names: refused on both; Force takes all of them. OTTY no
  longer does this.

The Force switch kept the default this ticket set, on, until the owner changed
it for Podman (see "Force starts off on Podman" below).

`Prune dangling` still counts images with no name that no container uses. The
engines' `dangling=true` list filter leaves out an image known only by its digest
(checked with the list call, no prune was run), so such a row is not counted.

### 2026-11-03: Force starts off on Podman

Owner decision, recorded in spec Decision 11. The switch in the image remove
dialog starts off on Podman and stays on on Docker. Force on Podman also deletes
the stopped containers that use the image, so one confirm would delete them.
Docker's Force deletes no container.

With the switch left off on Podman, the dialog says the engine refuses unless
Force is on, a confirm sends the call without Force, and the refusal reaches the
notice. Turning it on makes the dialog say the containers go too. The dialog
texts did not change. Nothing else changed: the "Stop it first" switch of a
container's remove dialog still starts on, because it only removes the container
the dialog names.
