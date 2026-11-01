# 14 — Run container form

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09, 12
Spec: [../spec.md](../spec.md)

## What

The form of spec Decision 18. Tests first. Screen: `DockerRun` (tweaks
`state`, `invalid`).

- Opens from "Run container" above the Containers table and from "Run" on an image (image filled in). Built from kit `FormPage`, `FormSection`, `TextField`, `TokenField`, `KeyValueField`, `Select`, `Segmented`, `Switch`.
- Form state and checks are pure and tested: image not empty, ports 1–65535, container paths absolute, a host path source exists. "Remove when it stops" forces Restart to No and disables the other choices.
- `otty-docker`: a `RunRequest` type mapped to bollard's create body (tested), then create and start.
- Flow: pull if missing and the switch is on (progress in the footer), create, start, show Containers with the new container selected. An engine error shows a `Notice` at the top and keeps every field.
- Right panel: what the image sets on its own (ports, command, entrypoint, workdir) from image inspect, or "Not on this computer yet".

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests: each check, the auto remove and restart rule, the form → request mapping (ports, env, binds and volumes, read only, network, restart policy, auto remove, command), the success and error flows.
- Manual: run nginx with a port and a bind mount on Docker and on Podman; `docker inspect` shows the same settings.

## Answer

"Run container" above the Containers table and Run on a tagged image open
the Run form in the main area; the right panel shows what the image sets on
its own. Tests were written first and seen failing (crate: 15 of 20 new
tests against stubs; kit: 9 of 12; app: 21 form and check tests, 13 flow
tests, 12 view model tests, 12 view tests), then the code.

### `otty-docker`

- `run.rs`: `RunRequest` (image, name, command, ports, env, volumes,
  network, restart, auto remove; builder `with_*` methods and getters),
  `PublishedPort` (host port or `None` for any, container port,
  `PortProtocol` tcp/udp), `VolumeMount` with `MountSource::HostPath` or
  `MountSource::Volume` and read-only, `RestartPolicy` (No, OnFailure,
  UnlessStopped, Always). `create_options()` puts the name in the query;
  `create_body()` maps to bollard's `ContainerCreateBody`: `Cmd`, `Env`
  (`NAME=value`), `ExposedPorts` and `PortBindings` (grouped per container
  port, so one port can be published twice), `Binds` (`src:dst[:ro]`, the
  text `docker run -v` sends for folders and named volumes),
  `NetworkMode`, `RestartPolicy`, `AutoRemove`. Auto remove always sends
  restart `no` (the engine refuses the pair).
- `connection/container_run.rs`: `Engine::run_container(&RunRequest)`
  creates, then starts, and returns the id. A failed start leaves the
  created container, as `docker run` does.
- `image_defaults.rs`: `ImageDefaults` (ports sorted, command,
  entrypoint, workdir, size) from image inspect;
  `Engine::image_defaults(reference)` is `Ok(None)` on 404.
- `fixtures::image_defaults()` from the hand-written
  `tests/fixtures/docker-image-inspect.json` (nginx-like, Docker 29 shape).

### Kit

- `MappingField` / `MappingRow` (`components/mapping_field.rs`,
  `.port-row`, `.vol-row`): column headings (only above rows), mono source
  → target inputs, a per-row control in a fixed-width column, a cross, an
  error line under the row, a ghost add button and a hint.
- `segmented_with_disabled`: the listed options are faded and send nothing
  (Restart while auto remove is on). `segmented` shares its builder.
- `FormPage::footer_bar`: a 60px bar fixed under the scrolling sections
  (`.form-foot`), so Run stays in reach.
- `KeyValueField::placeholders(name, value)` (Decision 15; `NAME` and
  `value` stay the defaults).
- `select_owned`: the kit `select` over options the dropdown owns (an
  engine's networks, the engines), built while drawing.
- Gallery: MappingField (ports with protocol and an error row; volumes
  with read-only), FormPage with the footer bar, a Segmented with disabled
  options, KeyValueField placeholders. `iced_test`: MappingField (shown
  parts in both themes, typing per row, cross, add, disabled row, control
  position), Segmented (disabled sends nothing, enabled picks), FormPage
  footer bar at the bottom, `select_owned` pick in both themes,
  KeyValueField placeholders.

### App (`app/src/widgets/docker/`)

- `run_form.rs`: `RunForm` (engine, image, name, pull switch on, command
  `TokenList`, port, env and volume rows, network, restart, auto remove,
  `ImageLookup`, `RunPhase` Editing / Pulling / Starting, notice),
  `RunEdit`, the auto remove rule (on sets No; other choices ignored while
  on), the lookup kept per engine and reference, `request()` (blank rows
  and nameless env rows left out, the word still typed in the command
  counts, `~` and absolute sources are folders), `parse_port`.
- `run_checks.rs`: `check(form)` → `RunProblems` (image required; host
  port empty or 1–65535; container port 1–65535; container path starts
  with `/`; a source is required; a folder source must exist).
- `run_flow.rs`: open from `OpenCreate` on Containers and from
  `RowAction::Run` (image name filled, looked up at once); an image change
  emits `LoadImageDefaults`; engine change sets its default network
  (`bridge`, else the first) and looks up again; `can_run` (editing, no
  problems, the engine answered); submit pulls first through the existing
  `StartPull` when the image is missing and the switch is on, else
  `RunContainer`; `PullProgressed` / `PullFinished` go to the Run form
  while it pulls and to the pull dialog otherwise; success closes the form,
  opens Containers of that engine without a filter, selects the new id and
  reloads (`LoadLists`); an engine error (pull, create or start) keeps
  every field and shows "Could not start web: …" (the image when no name);
  Cancel and opening another tree place close the form and stop a running
  pull; late answers are dropped.
- `run_model.rs`: `RunFormViewModel` (engine select only with two or more
  engines, image hint "On this computer, 192 MB" / pulled first / not
  pulled / checking, command hint with the image's command, "The image
  listens on …", row problems, networks, restart choices with the disabled
  ones and their hint, footer hint, pull line via `PullLineViewModel`, Run
  or "Running…") and `ImagePanelViewModel` (status badge here / missing /
  checking, Listens on, Command, Entrypoint, Workdir, lookup error).
- Views: `view/run_form.rs` (kit `FormPage` with footer bar,
  `FormSection`, `TextField`, `TokenField`, `MappingField`,
  `KeyValueField`, `Select`, `Segmented`, `Switch`, `Notice`, `Button`,
  the pull line of `form_dialog`); `view/run_panel.rs` (kit
  `InspectorHeader`, `StatusBadge`, `KeyValueList`, `Notice`). The root
  view shows them in place of the table and the inspector while the form
  is open.
- `actions.rs`: `RowAction::Run` first for tagged images;
  `action_model::can_create` is true for every list.
- `events/docker.rs`: `LoadImageDefaults` and `RunContainer` run through
  `Task::perform`. `guards.rs`: `RunFinished` and `ImageDefaultsLoaded` are
  background events.
- i18n: 53 keys in the ten locales; `Key::ALL` is 401. Helper
  `docker_filled(key, placeholder, value)`.

### Tests

Crate: the request body per field (bare request, name query, ports with
protocol and a free host port, one container port twice, env, folders and
named volumes with read-only, network, each restart name, auto remove
forcing `no`, command), run against the fake engine (create then start,
a refused create starts nothing, a refused start carries the message),
image defaults mapping and the 200 / 404 / 500 answers. App: every check,
the auto remove rule, rows and command editing, the form → request
mapping, lookup ordering, the flows above (ready run, pull then run,
failed pull, pull switch off, success shows the new container selected,
engine error keeps the form, cancel with and without a pull, another
place closes the form, pull dialog keeps its own results, engine change,
failed lookup), view models, `iced_test` views (sections in both themes,
Run enabled and disabled, Cancel, add rows, auto remove switch, disabled
restart choices, pull line, notice, typing), root view, event tasks,
guards, i18n templates.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 85.57% on
`905c25e` → 86.15%. Screenshots of the form (ready, with errors, pulling;
Dark and Light) were rendered with a throwaway `iced_test` and checked by
eye.

### Deviations and choices

- No live check against Docker or Podman; every engine call is tested on
  the fake engine. The manual check from "Done when" is still to do.
- Bind mounts get no SELinux label (`:z`): on Fedora with SELinux a
  folder may not be readable in the container. Adding `:z` relabels the
  user's files, so it is left for a decision.
- Restart starts at No (the engine default), not "Unless stopped" as on
  the board.
- Run waits for the engine's answer about the image (a lookup per edit of
  the image field; late answers are dropped). A failed lookup does not
  block Run.
- An empty host port lets the engine pick one; a volume row with a target
  and no source is an error ("Type a volume name or a folder").
- The footer is a fixed bar (new `FormPage::footer_bar`), as on the
  board; the launch editor keeps its in-page footer.
- A dangling image has no Run (it has no name to fill in).
- While the form is open, the logs of a container selected before keep
  following if its Logs tab was open (the subscription follows the table
  selection).

### For the next tickets

- 17/18: the status bar could say "Run container" while the form is open
  (the board does).
- Live check: run nginx with a port and a bind mount on Docker and on
  Podman and compare `docker inspect` / `podman inspect`.
