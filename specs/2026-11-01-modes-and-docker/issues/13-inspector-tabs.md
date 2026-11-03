# 13 — Inspector: Overview, Logs (follow), Inspect

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 03, 09
Spec: [../spec.md](../spec.md)

## What

Inspector tabs of spec Decision 12. Tests first. Screen: `DockerMode`.

- Tabs: containers have Overview, Logs, Inspect; the other resources Overview, Inspect (kit `InspectorTabs`).
- Overview: kit `KeyValueList` with the fields shown on the boards; empty fields are left out.
- Logs: a subscription keyed by engine and container id, alive only while Docker mode is active, the Logs tab is open and that container is selected. Starts with the last 500 lines, then follows. At most 5000 lines are kept. Kit `LogView`.
- Inspect: the engine's inspect JSON, pretty printed, loaded when the tab opens.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests: the logs subscription exists only in the right conditions and stops on tab change, selection change and mode change; the line cap; Overview fields per resource; Inspect loads once per selection.

## Answer

The inspector shows its tabs under the header: Overview, Logs and Inspect
for containers, Overview and Inspect for the other lists (kit
`InspectorTabs`). Tests were written first and seen failing in all three
crates (36 app tests, 4 kit tests, 9 `otty-docker` tests), then the code.

### `otty-docker`

- `inspect.rs`: `Engine::inspect_container(id)`, `inspect_image(id)`,
  `inspect_volume(name)` and `inspect_network(id)`, each one request, each
  returning the answer as pretty-printed JSON (`String`). The answer is read
  into bollard's model and written out again, so fields bollard does not know
  are left out.
- `logs.rs`: `Engine::follow_logs(id, tail) -> BoxStream<Result<String,
  EngineError>>` asks `follow=true`, `stdout`, `stderr` and `tail`. bollard
  splits the answer into stdout and stderr frames (or plain lines for a
  container with a terminal); they stay in the order they came and are cut
  into lines (`\r\n` and `\n`), stdin frames are dropped. An engine error ends
  the stream with that error.
- The 30 s request limit does not cut a long follow: in bollard 0.21.1 the
  limit wraps only the wait for the answer head (`execute_request`), not the
  body stream, so one client serves both (checked in the bollard source, not
  with a 30 s test).
- The two files hold their own `impl Engine` blocks, so tickets 11, 12 and 16
  can add calls to `connection.rs` without touching them; `Engine::client()`
  is a new crate-private getter for them.
- `Volume::compose_project()` (the `com.docker.compose.project` label; blank
  means none), for the volume Overview.
- `EngineAddress`, `EngineKind` and `EngineInfo` derive `Hash`: the logs
  subscription is keyed by engine and container id.
- `serde_json` is now a normal dependency (it was optional, behind
  `fixtures`; bollard already depends on it), to pretty-print the JSON. No
  new crate.

### Kit

- `LogViewState::max_lines(n)`: at most `n` lines are kept; the oldest shown
  and held-back lines are dropped first. `text_editor` cannot delete whole
  lines from the top without a layout, so once the limit is passed the text
  is rebuilt from the last `n` lines and the cursor moves to the end.

### App (`app/src/widgets/docker/`)

- `inspector.rs`: `InspectorTab` (with `of(resource)` and the translated
  label), `LogTarget` (engine + container id, the subscription key),
  `InspectTarget` (engine address, list, id), `InspectorState` (open tab, the
  followed `ContainerLogs` with its `LogViewState` and error, the last
  `InspectData`). `LOG_TAIL` = 500, `MAX_LOG_LINES` = 5000.
- `state.rs`: `DockerState::wanted_log_target()` (Docker mode active, Logs
  tab open, a container selected), `wanted_inspect_target()` (active, Inspect
  tab open, a row selected), `selected_row()` with `SelectedRow`, and
  `select_inspector_tab` (Logs only on containers). Opening another place goes
  back to Overview, as on the board.
- `reducer.rs`: after every intent `sync_inspector` follows the wanted logs
  (another target drops the old lines) and asks `LoadInspect` once per
  selection: the JSON is kept when the tab closes; the same row is not loaded
  again, a newly selected row is loaded when the Inspect tab is open. A late
  answer for another row, and lines of a container no longer followed, are
  dropped. New intents: `SelectInspectorTab`, `LogLines`, `LogsFailed`,
  `LogAction`, `InspectLoaded`, `InspectAction`; new effect `LoadInspect`.
  `DockerIntent` lost `Eq` (the `text_editor` action is only `PartialEq`).
- `logs.rs`: `Subscription::run_with(LogTarget, follow)`: connect, follow the
  last 500 lines, send the lines that are ready in batches of up to 500; a
  failure is sent once and ends the stream; the stream also ends when the
  container stops. `DockerWidget::subscription()` returns it only while
  `InspectorState::logs()` is set, which is exactly the wanted target, so it
  stops on tab, selection and mode change. `app/src/subscription.rs` adds it.
- `overview.rs`: fields per resource, empty ones (and "—") left out.
  Containers: Image, ID, Ports, Mounts ("source → destination"), Networks,
  Compose, Command, Created. Images: ID, Size, Used by (container names),
  Created. Volumes: Driver, Mountpoint, Compose, Used by, Created. Networks:
  Driver, Scope, Subnet, Gateway, Containers.
- `format.rs`: `date_time_text` ("2026-11-01 18:30 UTC", days to date with
  Howard Hinnant's `civil_from_days`, no new crate).
- `inspector_tabs.rs`: the tabs view model (`Overview(fields)`,
  `Logs { lines, error }`, `Inspect(Loading | Loaded | Failed)`).
- `view/inspector.rs`: kit `InspectorTabs`, `KeyValueList` (scrolls),
  the hint "Last 500 lines, new lines appear as they come." and kit `LogView`
  for the logs, a kit `Notice` for a failed stream or inspect, and a
  read-only `LogView` for the JSON, shown from its first line.
- `events/docker.rs`: `LoadInspect` connects and calls the matching inspect
  call through `Task::perform`.
- `guards.rs`: log lines, log errors and inspect answers pass open menus and
  inline edits like the other engine results.
- i18n: 12 keys in the ten locales (three tab names, the logs hint, two
  failure templates, six field labels; the other labels reuse the column
  keys). `Key::ALL` is 281.

### Tests

- Reducer: logs follow only the selected container on the Logs tab; they
  stop on tab change and start fresh on return, move to a new selection
  without old lines, stop on Leave and follow again on Enter, keep their lines
  over a reload that keeps the container, stop when the container is gone;
  Logs only on containers; another list goes back to Overview; at most 5000
  lines (5100 sent in two batches keep lines 100 to 5099); lines and errors
  of another container are dropped; a stream error shows and keeps the lines.
  Inspect loads once per selection when the tab opens, loads a new selection
  while open, loads a selection made on Overview when the tab opens, uses the
  right id per list, keeps the JSON and drops a late answer, shows a failure,
  and loads nothing outside Docker mode.
- Subscription (`mod.rs`, through iced's recipes and their hashes): one
  subscription only on the Logs tab of a container; none after another tab
  or Leave; a selection change replaces it with another key, the same
  selection keeps it.
- Overview fields per resource on the Docker fixtures, with fields left out;
  the tabs view model per resource; dates; the effect runs one task; guards;
  `iced_test` views: tabs and a tab press, Overview values, logs hint and
  error, inspect loading and failure, JSON in both themes.
- Kit: the line limit while following, a batch longer than the limit, held
  back lines while not following, empty lines.
- `otty-docker` against the fake engine: each inspect call (path and pretty
  JSON), an engine error with its message; logs with stdout and stderr frames
  in order and the query (`follow`, `stdout`, `stderr`, `tail=500`), a
  terminal container's plain lines, an engine error.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 83.83% on
`6b6a50f` → 84.12%. `cargo +nightly fmt`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo test --workspace
--all-features` and `cargo deny check` pass (deny shows only the yanked
warnings that were there before).

### Deviations

- Image Overview has no Platform, Exposes or Command (on the board): the
  image list does not carry them; they are in the Inspect tab.
- Dates are UTC with a "UTC" suffix (the board shows local time without a
  zone): OTTY has no time zone data and no date crate.
- Mounts do not say read-only.
- The logs view always follows: there is no follow switch on the board, so
  the user cannot scroll up while lines arrive (each new batch moves the view
  to the end). Lines split by the engine inside one frame are shown as two
  lines.
- A failed inspect is not retried for the same selection (retrying after
  every intent would loop); selecting another row and back loads it again.
- At the limit each new batch rebuilds the 5000-line text (see Kit).
- Not run against a live engine in this session; the visual look was checked
  only through `iced_test`.

### For the next tickets

- 11/12: the action buttons go into `view/inspector.rs` beside the header;
  `DockerState::selected_row()` gives the selected item. After a Remove, the
  reload drops the row and the inspector follows the new selection on its
  own.
- 15 (Shell): `SelectedRow::Container` and `LogTarget` already carry the
  engine and container.
- 16: an event reload that keeps the selected container keeps its logs; one
  that removes it stops them.

## Comments

### Logs after a restart (2026-11-03, spec Decision 12)

The logs went silent after a restart: the stream had ended, and the
subscription key (engine and container id) did not change, so iced never
started another stream. Reproduced on Docker before the fix.

Owner's decision: when the container comes back, clear the view and start
again from the new run. No separator, no old lines.

What changed:

- `otty-docker`: `WatchEvent::ContainerStarted(id)` for every `start` event of
  a container, never held back by the 500 ms reload gap. `WatchEvent` is no
  longer `Copy`.
- `otty-docker`: `Engine::follow_latest_run_logs(id, tail)` reads the
  container's `StartedAt` and `FinishedAt`, asks for lines with their times
  and drops those older than the middle of that gap. `since` alone is not
  enough: through bollard it takes whole seconds, and it brought back the old
  run's last lines after a fast restart.
- App: `DockerIntent::ContainerStarted` (a background intent in
  `is_docker_background`). When it names the followed container,
  `InspectorState::restart_logs` clears the lines and the old stream's error
  and counts one more comeback in `LogTarget`. The count is part of the
  subscription key, so iced drops the old stream and starts a new one. Late
  lines of the old stream carry the old key and are dropped. Follow keeps its
  state.

Seen on Docker 29.9.1 and Podman 5.8.8 (`k8s-file` and `journald` log
drivers), each log driver over `restart`, a restart through the API, stop and
start, and a crash loop with a restart policy:

- One `start` event per run on both engines.
- The stream ends on a restart, except Podman with `journald`, where it goes
  on with the new run's lines. Podman with `k8s-file` can deliver a few new
  lines before it ends.
- Both engines keep the old runs' lines in `/logs`.
- Line times against the run times: an old line can be stamped just after
  `FinishedAt` (Podman), a new line about 2 ms after `StartedAt` (Podman) and
  about 90 ms after it (Docker). The gap between the two was 60 to 100 ms for
  a restart, so its middle divides the runs.

Not covered: a start that happens while the events stream is broken (for
example the engine itself restarting) is missed, so the logs stay silent until
the next start or until the tab or selection changes.
