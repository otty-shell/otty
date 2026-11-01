# 26 — Docker tables: sorting, defaults and kept choice

Status: resolved
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 24, 25
Spec: [../spec.md](../spec.md)

## What

Spec Decision 19, second point. Tests first.

- Sort state per table (resource kind): column and direction, kept in the
  widget state while OTTY runs (not saved to disk).
- Defaults: containers running first (then restarting, paused, created,
  exited, dead), then by name; images, volumes, networks by name.
- Header press: a new column sorts in its natural first direction (text A→Z,
  ages and dates newest first, sizes and counts largest first, status by the
  state order above); pressing the same column again reverses it.
- Sorting is a pure, tested function per resource over the shown rows; the
  filter and compose filter still apply; selection stays on the same item
  (by id) after a sort.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure sort tests per resource and column, both directions, ties broken by
  name; default orders.
- Reducer tests: header press sets and reverses; selection kept by id.
- `iced_test`: header press emits the sort intent; the arrow shows.

## Answer

Tests were written first against stubs (12 of the 13 new sort tests failed),
then the code. No new text, so no i18n change.

- New `app/src/widgets/docker/sort.rs`: `SortOrder` (value order: Ascending
  is A to Z, smallest first, oldest first), `TableSort` (column + order),
  `TableSorts` (one per table, `Default` = the defaults), `is_sortable`,
  and the pure `sort_containers/images/volumes/networks(rows, sort)`. Ties
  always go by name ascending, whatever the direction.
- Defaults: containers by Status ascending (running, restarting, paused,
  created, exited, dead, rest) then name; images, volumes, networks by the
  name column.
- Header press (`TableSorts::pressed`): the same column reverses; a new
  column starts in its natural way: text (and Status) ascending; Age,
  Size and the counts (images "Used by", networks "Containers") descending
  (newest / largest first). The Ports column is the only column that cannot
  be sorted (`is_sortable`; `columns()` marks the rest `.sortable(true)`).
- `DockerState` keeps `sorts: TableSorts` (memory only), `sort(resource)`
  and `sort_by(column)`; new intent `SortBy(column)` sorts the open table.
  The selection is the item id, so it stays on the same item; only its
  shown position moves.
- One ordering for everything that maps a position to a row:
  `shown_containers` (now takes the sort), `shown_images`, `shown_volumes`,
  `shown_networks` in `types.rs` are used by the view model,
  `select_shown_row` (row press and right press) and
  `open_compose_project_of_row`, so the filter and the compose filter still
  apply and a pressed row is the row that is shown.
- `TableViewModel` gains `sort`; the table view calls
  `.sorted_by(column, direction)` and `.on_sort(DockerIntent::SortBy)`
  (Ascending maps to the kit's up arrow, Descending to the down arrow).
- Existing tests that pressed rows by engine position now use a test-only
  `DockerState::shown_position(engine_index)`, because the default order is
  no longer the engine's order.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.43%
(63784 lines, 8658 missed) to 86.56% (64453 lines, 8665 missed).
