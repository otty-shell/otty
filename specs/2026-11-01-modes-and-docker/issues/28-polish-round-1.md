# 28 — Polish: no mode name, row ⋯ menu at its button, sort arrow, inspector text

Status: ready-for-agent
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
