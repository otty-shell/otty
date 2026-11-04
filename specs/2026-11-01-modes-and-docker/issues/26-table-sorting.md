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
  (newest / largest first). Every column can be sorted (`is_sortable`;
  `columns()` marks them all `.sortable(true)`). The Ports column could not be
  at first; see "Ports sorts as text" under Comments.
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

## Comments

### One sort per page, text ignores case (2026-11-04, spec Decisions 19 and 21)

The sort was kept per resource, so a sort on Docker's Containers also ordered
Podman's. `TableSorts` is now a map from `Place` to `TableSort`, apart from the
engines, so a page keeps its sort while its engine is away. A page nobody sorted
has no entry and gets its default.

Decision 19 always said case-insensitive, but every text column used plain `cmp`,
so capitals came first. All text columns and the name tie-break now go through
`otty_docker::compare_text` (`crates/docker/src/text_order.rs`), and so do the
names listed inside one cell: the users of a volume, a network and an image, and
the networks of a container. It compares iterators of lowered chars, so nothing is
allocated per comparison, and falls back to plain `cmp` so names that differ only
in case keep a fixed order. Numeric, size and date columns are unchanged. In the
Tag and Repository columns `<none>` comes after text that starts with a digit and
before text that starts with a letter, ascending.

### Ports sorts as text (2026-11-04, spec Decision 19, owner decision)

Decision 19 says every table can be sorted by pressing a column header, but the
Ports column was left out. It is now on, sorted simply as text: `sort_containers`
calls `compare_text` on `ports_text`, the function the cell itself uses, so several
ports are compared as the cell joins them (`8080:80/tcp, 5432:5432/tcp`) and a port
Docker lists once per address family counts once. There is no numeric order, so
`10000:80/tcp` comes before `80:80/tcp`. The first press is ascending and rows that
read the same follow the name, like the other text columns.

There is no special rule for containers without ports either. Their cell shows the
dash `—`, which comes after the digits, so they come last ascending and first
descending. This differs from Compose and Subnet, whose rows with no value come
first because those columns compare the value and not the shown dash.

`ports_text` allocates, and `sort_containers` has no step that could build a key
once per row, so the text is built for each comparison. For 148 rows that is about
1,080 comparisons, so about 2,160 calls, and about 1.1 ms per sort in a debug build
against 0.6 ms for the Image column. Only a table sorted by Ports pays this.
