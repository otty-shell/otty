# 36 — Narrow window: tree labels and short table columns stay readable

Status: resolved
Type: bug
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

Seen on the 900 px snapshot of ticket 19 (`12-narrow-overview`), spec
Decision 19 (layout for small screens):

- **Tree rows.** `TreeRow` gives the count the room first, so "Containers"
  with "2 running" is cut to "Co", and the engine root loses its last digit
  ("Docker 29.9"). The label comes first: it keeps its width and ends with
  "…" only when it truly does not fit, and the count or note gives way first
  (it shrinks with "…" or hides when too little room is left). Use the kit's
  `EllipsisText`. Other callers of `TreeRow` (Files, Launches) keep working.
- **Table columns.** All columns shrink by share, so short columns become
  useless: "Age" reads "A…" with values "1…", and ports read "18080:8…". A
  column can state a minimum width (`TableColumn::min_width`) that it never
  goes below; the columns with no minimum give way first. Set minimums in the
  app for short columns (Age, Status, ID, Size, Used by, Driver, Scope,
  Containers) and enough for one port on Ports. When even the minimums do not
  fit, the table scrolls sideways with the thin kit scrollbar, and the header
  scrolls with it, instead of cutting. The row "⋯" column stays reachable by
  scrolling.

Tests first. Gallery updates in both themes. Run fmt, clippy, tests, `cargo
deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test`: a `TreeRow` at a narrow width keeps its whole label and
  cuts or hides the count; a very narrow one ends the label with "…".
- Kit `iced_test`: a column with a minimum keeps it at a narrow width while
  others shrink; below the sum of minimums the table scrolls sideways and the
  header moves with the rows; row, link and "⋯" presses still hit the right
  row and column after a sideways scroll.
- A snapshot of the 900 px window shows "Containers" and "Age" readable.

## Answer

Tests were written first and seen failing (8 kit tests: the width split,
the minimum kept, the header moving with the rows, and the two tree row
cases). Then the code.

### Kit (`otty-ui-kit`)

- `TreeRow`: the label is an `EllipsisText` that takes the room it needs
  first. The state word and the count are now one text on the right
  ("running 2"), which takes what is left and ends with "…" or shows
  nothing when the room is gone. Other callers are unchanged.
- `TableColumn::min_width(u16)` (pixels, side padding included, default 0).
  A private `column_widths` splits the width by shares; a column below its
  minimum keeps the minimum and the rest share what is left, repeated until
  stable. When the minimums do not fit, columns are their minimums (a column
  with no minimum is 0 wide, so give every column a minimum).
- `DataTable` is built inside `responsive`, so it knows its width. When the
  minimums do not fit, header and rows sit in one horizontal `scrollable`
  (new `scrollbar::horizontal()`, same thin style). New private
  `table_sideways.rs` (`SidewaysScroll`) reads the scroll offset before each
  event; `TableBody` takes it out of right-press and "⋯" points, so the row
  menu still opens at its button. Handlers became `Rc` for this.
- Gallery: a "minimum widths" section (520 and 320 wide) and a "TreeRow
  narrow" section (200, 150, 90 wide), in both themes.

### App

Minimums in `resource_rows::columns()` (px): Containers Name 110, Status
104, Image 96, Compose 80, Ports 130 (one port), Age 52 (sum 572, fits the
576 px a 900 px window leaves); Images 140, ID 118, Size 76, Used by 76, Age
52; Volumes 140, Driver 76, Used by 110, Age 52; Networks Name 120, Driver
76, Scope 72, Subnet 124, Containers 100, Age 52. Tests of the columns
updated.

Coverage (lines): 87.54% (68517 lines, 8536 missed) -> 87.59% (68968 lines,
8559 missed). fmt, clippy, workspace tests and `cargo deny check` pass.

### Notes

- Snapshot of the 900 px window (throwaway, not committed):
  `scratchpad/shots36/narrow-dark-wgpu.png` and `narrow-light-wgpu.png`.
- When the table scrolls sideways the vertical scrollbar sits at the right
  end of the wide content, so it shows after scrolling right.
