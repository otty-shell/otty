# 28 — Polish: no mode name, row ⋯ menu at its button, sort arrow, inspector text

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 27
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20 (layout points and "Polish from round 1").

- Remove the active mode's name next to the mode switcher (all modes).
- The row "⋯" menu opens at the pressed button (check the point the kit
  `DataTable::on_row_menu` reports against the root view's coordinates and
  `anchor_position`; a snapshot of ticket 27 showed it in the middle of the
  table).
- `DataTable`: the sort arrow sits right after the column title, also for
  right-aligned columns (arrow before the title there).
- Inspector: one side padding for header, buttons, hint, tabs and body; the
  status line and hint wrap instead of being cut; `KeyValueList` values wrap
  at spaces, `/`, `-`, `:` and `.` before falling back to any character;
  dates read `2026-11-01 18:30 UTC`.

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: no mode name text in the switcher row; the row menu's
  position equals the button's corner in the root view; arrow position in a
  header; a long port value wraps at the slash; status text wraps.

## Answer

Tests were written first and seen failing (the switcher, row menu and
sort arrow tests failed on assertions; the status badge test did not
compile until `StatusBadge::wrapping` existed), then the code.

- **No mode name:** the switcher row shows only the kit `ModeSwitcher`
  (`widgets/modes/view/mode_switcher.rs`). `ModesViewModel::active` stays:
  the root view uses it to pick the panels' content.
- **Row "⋯" menu at its button:** the kit point was already right (the
  button's bottom-left corner in window coordinates; a root-view test
  confirms it). The menu was wrong because the "⋯" column is the last one:
  `anchor_position` flipped the menu to the left of that corner, into the
  middle of the table. `main_area` now sends the button's bottom-right
  corner in `RowMenuPressed`, so the menu opens under the button and ends
  at its edge (8 px margin). Right click is unchanged.
- **Sort arrow:** `EllipsisText::shrink(reserved)` makes a title as wide as
  its text (and shortens it with "…" to leave room for the arrow). The
  header puts the arrow right after the title, or before it in a
  right-aligned column.
- **Wrapping:** new kit-private `WrappedText` (`wrapped_text.rs`) breaks
  after a space, `/`, `-`, `:` or `.` when it can and between characters
  only for a piece wider than the line; `KeyValueList` values use it.
  `StatusBadge::wrapping(true)` lets the label wrap (the table keeps one
  line); the inspector status line uses it. The hint under the buttons
  already wrapped; a test now guards it.
- **Padding:** header, buttons, hint, tabs and body already start at the
  same 16 px side padding; a layout test now guards it.
- **Dates:** the Overview already shows `2026-11-01 18:30 UTC`
  (`format::date_time_text`, tested); with the new wrapping the date breaks
  at its spaces.
- Gallery: the inspector section shows a long wrapping status, a ports
  value that breaks at the slash and a short date.
- No new text, so no i18n change.

Tests: `wrapped_text` (break after `/`, spaces, `-` `:` `.`, fallback to
characters, spaces at a line end, new lines, multibyte); `DataTable` (title
width and position with the arrow, left and right aligned, long title keeps
room for the arrow); `StatusBadge` (wraps / stays on one line);
`KeyValueList` (port value wraps inside its width); inspector (status and
hint wrap, one side padding); modes switcher (no name in any mode); root
view (row "⋯" reports the bottom-right corner; the opened menu sits under
the button; no mode name in Terminal mode).

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.64%
(64846 lines, 8664 missed) on `475e4a7` -> 86.71% (65309 lines, 8681
missed).
