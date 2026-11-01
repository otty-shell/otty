# 11 — Container actions, confirm dialogs, notices

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Container actions of spec Decision 11. Tests first. Screen: `DockerMode`
(tweaks `confirm`, `notice`).

- `otty-docker`: start, stop, restart, pause, unpause, remove (with force) and prune stopped.
- Inspector buttons by state — running: Shell (ticket 15), Restart, Stop, Pause, Remove; paused: Unpause, Stop, Remove; restarting: Stop, Remove; exited or created: Start, Remove.
- The same actions in a context menu on right click on a row (kit `Menu`).
- Remove and Prune ask first (kit `Dialog`); Remove of a running container has the switch "Stop it first", on by default.
- While an action runs its buttons are disabled. A failure shows a kit `Notice` with the engine's message; success reloads the containers list.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: each action's effect, the button set per state, the confirm flow (cancel and confirm), failure shows a notice, dismiss hides it.

## Answer

Container actions work from the inspector buttons and from a row menu;
Remove and Prune ask first. Tests were written first and seen failing (the
new calls, intents and keys did not exist), then the code was added.

### `otty-docker`

- `connection/container_actions.rs` (a child module of `connection`, so it
  uses the private client; no change to the `Engine` struct):
  `start_container`, `stop_container`, `restart_container`,
  `pause_container`, `unpause_container`, `remove_container(id, force)` and
  `prune_containers()` (no filters: every created, exited or dead
  container). One request each, `Result<(), EngineError>`.
- `EngineError::message()`: the engine's own words for a refused request
  (`DockerResponseServerError`, `DockerStreamError`), otherwise the whole
  error text. The notice shows it, so it does not start with "Docker
  responded with status code 409" (wrong for Podman too).
- `test_support::FakeEngine`: a unix socket server that answers requests
  in order and records the request line and body (`read_request`). Tests:
  the method and path of every call, `force=true` only when asked, prune
  without filters, and a 409 answer whose `message()` is the engine text.

### App (`app/src/widgets/docker/`)

- `actions.rs`: `RowAction` (Start, Stop, Restart, Pause, Unpause,
  Remove), `container_actions(state)` (running: Restart, Stop, Pause,
  Remove; paused: Unpause, Stop, Remove; restarting: Stop, Remove; exited or
  created: Start, Remove; removing: none; dead and the rest: Remove),
  `needs_stop(state)` (running, paused, restarting), `EngineAction` (one
  engine call, with its list, target id and failure key), `ActionRequest`
  (engine, call, row name), `Confirm` (request, optional switch, prune
  count) and `failure_text`.
- `action_flow.rs`: the reducer side. `Act(action)` closes the row menu and
  runs the action on the selected container only when its state offers it
  and no action runs on it; Remove opens a `Confirm` with "Stop it first"
  on for running, paused or restarting containers. `Prune` counts the
  stopped containers and asks. `ConfirmAccepted` applies the switch (on =
  `force`) and runs. `ActionFinished` forgets the request; success emits
  `LoadLists` for the engine (only while Docker mode is active), a failure
  sets the notice ("Could not stop web-1: <engine message>") and leaves the
  table as it was. `OpenRowMenu { index, position }` selects the row and
  opens the menu (nothing past the last row).
- `state.rs`: running requests, the open `Confirm`, the row menu position
  and the notice; `is_busy(engine, list, target)`. Opening another place
  hides the notice (as on the screen).
- `action_model.rs`: `ActionsViewModel` (buttons, all disabled while their
  container is busy; a `hint` field for ticket 12), `RowMenuViewModel`,
  `ConfirmViewModel`, `PruneViewModel` ("Prune stopped", disabled while the
  prune runs). `TableViewModel` gained `prune` and `notice`.
- Views: `view/inspector.rs` puts kit `Button`s (small; Remove in the
  danger kind) into `InspectorHeader::action` and the hint into
  `InspectorHeader::hint`; `view/main_area.rs` adds the ghost Prune button
  before the create button, the action notice with a dismiss cross (kit
  `Notice::on_dismiss`) and `DataTable::on_row_right_press`;
  `view/row_menu.rs` (kit `Menu`, anchored with `geometry::anchor_position`
  inside the window, dismiss layer, `opaque` so a disabled item does not
  close it); `view/confirm_dialog.rs` (kit `Dialog`, muted text, kit
  `Switch`, ghost Cancel, danger action).
- `view.rs`: the row menu and the dialog are layers over the whole window
  in Docker mode, because the table reports the right press in window
  coordinates; the resize grips step aside while they are open.
- `events/docker.rs`: `RunAction(request)` → `Task::perform` of the engine
  call → `ActionFinished { request, result }` with `EngineError::message`.
- `guards.rs`: `ActionFinished` passes open menus and inline edits like the
  other engine results; action clicks close menus like other clicks.
- `DockerIntent` lost `Eq` (it carries an `iced::Point`); nothing needed it.
- i18n: 21 keys in the ten locales (action labels, "Prune stopped",
  "Prune", remove and prune dialog texts, "Stop it first", "Dismiss", seven
  failure templates); `Key::ALL` is 290. Helpers `docker_named` and
  `docker_action_failed`.

### Tests

Actions: offered actions per state, danger, `needs_stop`, list and target
of each call, `acts_on`, the switch forcing removal, a confirmation without
a switch, failure texts, one failure key per call. Reducer (`action_flow`):
each direct action's effect, an action the state does not offer, a busy
container takes no other action while another can act, Remove asks and
Cancel runs nothing, "Stop it first" on by default (forced) and off (not
forced), a stopped container has no switch, Prune asks with the count of
stopped containers, a second prune waits, a confirmation without a dialog,
success reloads, failure shows the notice and keeps the table, dismiss and
opening another list hide it, no reload outside Docker mode, the row menu
(select and open, item runs and closes, close runs nothing, past the last
row). View models: buttons per state, disabled while busy, row menu items
and position, remove and prune dialogs, switch state, Prune button. Views
(`iced_test`): inspector buttons send `Act`, a disabled button and its hint;
Prune, disabled Prune, notice dismiss, right press with the pointer; row
menu item, disabled item, press outside; dialog title, text, switch, Cancel,
action, no switch. Root view: the row menu item over the table and the
remove dialog. Guards, events, i18n templates in every locale.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 83.83% on
`6b6a50f` → 84.34%.

### Deviations and choices

- "Stop it first" sends `force=true` (the engine kills the container, then
  removes it; `docker rm -f`), as the ticket's "remove (with force)" says,
  not a separate stop request. Volumes are kept (`v=false`).
- Shell is not among the running buttons yet: ticket 15 adds it to
  `container_actions(Running)` (first, as on the screen) and its handling in
  `action_flow::act`.
- No live check against a real engine was run; every engine call is tested
  against the fake engine.
- Esc does not close the row menu or the dialog; the dismiss layer and
  Cancel do.

### For the next tickets

- 12: add image, volume and network calls to `EngineAction` and their
  buttons to `action_model::action_buttons` (the `hint` field is ready);
  `prune_button` returns `None` for the lists without a Prune today.
- 14 (Run): the create button above the containers table is still
  disabled; enable it in `view/main_area.rs::header_row`.
- 15 (Shell): see above; the button is disabled with a hint when the CLI is
  missing (`ActionsViewModel::hint`).
- 16 (events): after an action the lists are reloaded once by
  `ActionFinished`; the engine events will also arrive and reload them.
