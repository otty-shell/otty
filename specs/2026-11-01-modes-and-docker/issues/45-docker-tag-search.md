# 45 — Docker tables use tag search; compose becomes a tag

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 44
Spec: [../spec.md](../spec.md)

## What

Spec Decision 21. Tests first.

- Each resource gives the schema: keys in column order (Decision 21 names; the
  column `Repository:Tag` is `repository`), names, values taken from the shown
  cell text.
- The toolbar's field is `TagSearchField`; `TagRow` sits between the toolbar and
  the table. The name filter and the compose filter become this one mechanism:
  remove `Place::compose_project`, the chip next to the title and the old filter
  text from the state.
- Tags kept per table in the widget state, like the sort; the field's text is not
  kept when another list opens.
- The Compose link adds `compose:<name>`, replacing another `compose` tag.
- `show_item` (links) removes the target table's tags only if they hide the item.
- `shown_*`, row presses, the row menu, port links and the empty message follow
  the new filter. The empty message no longer assumes a name filter.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: tags per table survive switching lists and back; Clear all and
  a single cross; the compose link replaces; a link to a hidden item removes the
  tags and one to a visible item keeps them.
- Pure tests: schema per resource; several ports; AND and OR over the fixtures.
- `iced_test`: the full flow `sta` Enter, `run` Enter gives the tag, a cleared
  field and fewer rows; a row press after filtering hits the right row.
