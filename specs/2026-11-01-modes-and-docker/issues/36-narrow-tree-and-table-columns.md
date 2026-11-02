# 36 — Narrow window: tree labels and short table columns stay readable

Status: ready-for-agent
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
