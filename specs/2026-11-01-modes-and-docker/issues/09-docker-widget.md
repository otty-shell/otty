# 09 — `docker` widget: engines, tree, tables, selection, filter, Refresh

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 02, 06, 08
Spec: [../spec.md](../spec.md)

## What

New widget `app/src/widgets/docker/` (spec Decisions 5, 8, 10). Tests first.
Screens: `DockerMode`, `DockerImages`, `DockerVolumes`, `DockerNetworks`.

- State: engines and their lists with a load status per list; the open place (engine, resource, compose filter); selected row per resource (by id); filter text.
- Entering Docker mode runs discovery (ticket 07) and loads all lists (`Task::perform`); Refresh reloads all lists. Nothing runs while another mode is active.
- Left panel: one root per engine ("Docker 27.3", "this computer"), children Containers ("N running"), Images, Volumes, Networks with counts. Compose projects come in ticket 17.
- Main area: title, engine chip, filter field, the create button (disabled until tickets 12 and 14), Refresh, and the kit `DataTable` with the columns of Decision 10 and the badge tones.
- Right panel: kit `InspectorHeader` of the selected row (title and status); tabs and buttons come in tickets 11–13.
- Age text ("2 min", "2 h", "3 d", "2 w", "1 mo", "1 y") and size text are small tested helpers.
- Selection survives a reload when the id is still there and moves to the first row otherwise.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: open each resource, filter, reload keeps or drops the selection, a failed list shows its error and keeps the other lists.
- View model tests: columns and badge tones per resource; counts in the tree; age and size helpers.

## Answer

New widget `app/src/widgets/docker/` (state → reducer → view models →
views → intents). This ticket changes `app/src/view.rs`.

### Files

- `types.rs`: `ResourceKind` (Containers, Images, Volumes, Networks, with
  title, create and empty-table keys), `Place` (engine address + resource),
  `LoadedLists` (four `Result<Vec<_>, String>`, one per list), the name each
  row is filtered by (`image_name` gives the first tag or `<none>:<none>`)
  and `filtered` (case-insensitive part of the name).
- `state.rs`: `DockerState` (is active, `Search` = `Searching` or
  `Done(Discovery)`, engines, open place, filter), `EngineState` (the
  `EngineInfo` and four `ResourceList`s), `ResourceList<T>` (a `ListLoad`
  = `Loading` / `Loaded(Vec<T>)` / `Failed(String)`, and the selected id).
  Ids: container id, image id, volume name, network id.
- `event.rs`: `DockerIntent` = `Enter`, `Leave`, `DiscoveryFinished`,
  `ListsLoaded { engine, lists }`, `Open(Place)`, `SelectRow(index)`,
  `FilterChanged`, `Refresh`. `DockerEffect` = `Discover`,
  `LoadLists(EngineInfo)`.
- `reducer.rs`: `Enter` marks the mode active and asks `Discover`; `Leave`
  marks it inactive. `DiscoveryFinished` keeps the lists and selection of
  engines found again, adds new ones, drops gone ones, keeps the place when
  its engine is still there (else the first engine's Containers), and asks
  `LoadLists` for every engine only while active. `Refresh` does the same
  (only while active). `ListsLoaded` applies each list on its own; results of
  an engine that is gone are dropped. `Open` changes the place and clears the
  filter (not when the same place is opened again). `SelectRow(i)` selects
  row `i` of the shown, filtered table.
- `model.rs`: three builders, one per slot, so each slot builds only what it
  shows: `tree_rows` (engine roots "Docker 29.9.1" with the local dot and
  "this computer"; the open engine's four lists (since 2026-11-03 the lists
  of every engine that is not closed, see Comments), Containers with "N running",
  the others with a count, nothing while loading, "failed" in the failed
  tone), `main_area` (`Searching`, `NoEngine` or `Table` with columns, rows,
  selected position, empty message and error) and `inspector` (title and
  status of the selected row). `engine_label` is reusable by ticket 18.
- `resource_rows.rs`: columns (Decision 10, numbers right-aligned), cells,
  badge word and tone (running ok; paused, restarting warn; exited non-zero,
  dead error; created, exited 0 and the rest neutral) and inspector status
  lines (container: the engine's status text in the badge tone; image: used
  by N / dangling (warn) / not used; volume: used by names / not used (warn);
  network: built in / N connected).
- `format.rs`: `age_text` ("< 1 min", "N min", "N h", "N d" below 14 days,
  "N w" below 30 days, "N mo" below a year, "N y"), `size_text` (SI units,
  three significant digits: "13.3 kB", "6.69 MB", "1.12 GB"), `short_id`
  (drops `sha256:`, 12 characters), `ports_text` ("18080→80/tcp"; Docker's
  `0.0.0.0` / `::` pairs show once; "—" for none).
- `view/engine_tree.rs` (kit `TreeRow`s in a scrollable column),
  `view/main_area.rs` (header row: title, kit `Chip` with the engine, kit
  `TextField` filter, disabled kit `Button` for the create action, kit
  `IconButton` Refresh; a kit `Notice` when the list failed; kit
  `DataTable`; a kit `EmptyCard` while searching or without an engine),
  `view/inspector.rs` (kit `InspectorHeader` with a `StatusBadge`),
  `view/status_bar.rs` (the old placeholder status bar, unchanged).

### Wiring

- `app/Cargo.toml`: `otty-docker` path dependency; as a dev-dependency with
  the new `fixtures` feature.
- `events/docker.rs`: `Discover` → `Task::perform(otty_docker::discover())`
  → `DiscoveryFinished`. `LoadLists(info)` → `Engine::connect`, then
  `containers()`, then `images`/`volumes`/`networks(&containers)` at the
  same time (`join3`), each keeping its own error → `ListsLoaded`.
- `events/modes.rs`: `ModeChanged(Docker)` → `DockerIntent::Enter`;
  `ModeChanged(Terminal)` → grid sync, focus and `DockerIntent::Leave`.
- `view.rs`: left panel `Mode::Docker` → `engine_tree`; main area →
  `main_area` (ages against the current time); right panel → `inspector`;
  status bar → `docker::view::status_bar`. The `_` arms are gone.
  `modes/view/docker_placeholder.rs` and `docker_placeholder_description`
  are deleted.
- `guards.rs`: Docker mode changes, engine results and effects pass open
  menus and inline edits; Docker clicks close menus like other clicks.
- `crates/docker`: feature `fixtures` (`otty_docker::fixtures::engine`,
  `resources`, `discovery`) gives real values from the captured answers,
  since the plain types have no public constructors. `serde_json` (already a
  workspace dependency) is optional behind it. `Resources::from_answers`
  now holds the mapping that `resources()` did inline.
- 58 i18n keys in the ten locales, `with_count`, `docker_exited`,
  `docker_nothing_matches`, `docker_list_failed`, `docker_used_by`.
- Icons `assets/svg/refresh.svg`, `assets/svg/notice-error.svg`.

### Tests (written first, seen failing)

Reducer: enter searches; found engines load and open the first engine's
containers; nothing loads after Leave (search result, Refresh); no engine
clears engines and place; a new search keeps lists and place; first rows
selected; opening each resource clears the filter; a row press selects in
the filtered table, for each resource, past the end changes nothing; a
reload keeps the selection or moves to the first row; a failed list shows
its error and keeps the others; results of a gone engine are dropped;
Refresh reloads every engine; Leave keeps what is shown. View model: tree
rows and counts for two engines, loading and failed notes, every table's
columns and rows, filter with selected position, selection outside the
filter, nothing matches, loading and empty messages, failed list, inspector
per resource, engine label. Rows: columns per resource, badge tones and
words, cells per resource, status lines. Format: age buckets and text, sizes,
short id, port pairs. `iced_test`: tree row press opens its list, table row
press selects, filter typing, disabled create, failed notice, cards, the
root view in Docker mode (click a table row). Guards and `events/modes.rs`
(nothing talks to the engine in Terminal mode).

Coverage (lines): 82.66% on `6b03b57` → 83.36%.

Screens were rendered offscreen with `iced_test` (root view, fixture data,
Containers, Images, Volumes, Networks in OTTY Dark and Light); the
throwaway test was not committed. The live app against a real engine was
not run in this session.

### Deviations

- Lists load as containers first, then the other three, instead of
  `Engine::resources()`, so one failed list keeps the others (the ticket's
  "a failed list shows its error and keeps the other lists"). If containers
  fail, "used by" of the others counts from an empty list.
- The compose filter of the place is left to ticket 17 (no unused field).
- The filter is the kit `TextField` with no label: 34 px tall and mono
  (design: 28 px, UI font). The create button has no "+" icon (kit
  `Button` has none).
- A failed list shows a `Notice` without dismiss above an empty table, and
  its rows are dropped; the selected id is kept for the next load.
- Overlapping loads (Refresh twice) are not ordered: the last result wins.
- In the 260 px panel the root label is cut ("Docker 29.9.") because
  `TreeRow` has no ellipsis and "this computer" takes the space.

### For the next tickets

- 10: `Search::Done(discovery)` with no engines → `MainViewModel::NoEngine`;
  replace the `NoEngine` card in `view/main_area.rs` (`card`) and the
  `docker_no_engine` key. Check again = `DockerIntent::Enter`-like intent
  (`start_search` + `Discover`).
- 11/12: add intents for the actions; the inspector buttons go into
  `view/inspector.rs` as `InspectorHeader::action(..)`; the context menu
  from `DataTable::on_row_right_press` in `view/main_area.rs`; enable the
  create button there. After an action, emit `LoadLists(info)` (or a
  per-list load). The `Notice` and `icons::NOTICE_ERROR` are already used.
- 13: tabs under the header in `view/inspector.rs`; the selected row is
  `ResourceList::selected(id_of)`.
- 16: subscription keyed by `state.is_active()` (promote
  `DockerWidget::is_active` out of `#[cfg(test)]`) and the engines; map an
  event to a list load in `events/docker.rs`.
- 17: add the compose project to `Place`, its tree rows in
  `model::tree_rows`, the filter in `model::main_area` (`table`) and the
  chip in `header_row`.
- 18: replace `view/status_bar.rs`; `engine_label` and `size_text` exist.

## Comments

### Engines open and close like folders (2026-11-03, spec Decision 10)

With Docker and Podman on, pressing the Podman root hid Docker: `tree_rows`
took "open" from the place, so only the engine of the shown table was ever
open, and a press on a root also changed the table. The owner wants engines to
work like folders.

- `DockerState` keeps the closed engines (`closed_engines`), not the open ones,
  so every engine starts open, also one found later. Memory only: it survives a
  mode switch and resets when OTTY restarts.
- A press on a root sends the new `DockerIntent::ToggleEngine(address)`. It
  changes nothing on the right and loads nothing. A press on a list row still
  sends `Open(place)`.
- Closing the engine of the shown table leaves the table. No row is highlighted
  then.
- `DockerState::open` removes the engine of the place from the closed set, and
  so does `finish_search` when it moves the place. These are the only two
  places that set the place: inspector and table links and a finished Run all
  go through `open`.
- Lists of every engine load at Enter and on Refresh and follow `ListChanged`
  already (nothing there depended on the shown engine), and a closed engine
  keeps loading too. Tests now cover this for the engine that is not shown.

The "open engine's four lists" in the Answer above describes the old behaviour.

### A closed engine stays closed when it comes back (2026-11-04, spec Decision 10)

Owner decision. `finish_search` removed the engine from `closed_engines` whenever
the place moved, so an engine the user closed opened again after the no-engine
card. Now the engine opens only when the place moves from a lost engine to another
engine that is still there, so the user can see whose list is on screen. From an
empty place (the card, or the first search) the closed state is kept: the table
shows the containers and the tree row stays closed. The test
`a_closed_engine_found_again_after_no_engine_opens_with_its_place` was changed on
purpose to `a_closed_engine_found_again_after_no_engine_stays_closed_and_is_shown`.
The Comments above saying `finish_search` opens the engine when it moves the place
describe the old rule.
