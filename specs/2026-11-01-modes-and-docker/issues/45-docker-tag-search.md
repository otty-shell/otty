# 45 — Docker tables use tag search; compose becomes a tag

Status: resolved
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

## Answer

The Docker tables now use the kit's tag search. The widget's part is the new
`table_search.rs`: the keys of each table in column order (`keys`), one
`SearchRow` per item built with the same formatters as the cells, one schema
function per table, and `TableSearches`, which keeps one `TagSearch` per table
like `TableSorts` keeps the sort. A test compares each row's values with the text
of its cells, for every fixture item of every table.

- **Layout:** title row, then the toolbar with `TagSearchField`, then `TagRow`
  (only with tags), then the table. The field grows to 320 px, up from 240, for a
  typed `key:value`.
- **One mechanism:** `Place::compose_project`, `Place::compose`, the chip next to
  the title, `DockerState::filter`, `FilterChanged` and `ClearComposeProject` are
  gone. Every user action arrives as `DockerIntent::TagSearch(TagSearchAction)`.
- **`shown_*`** take the search, so row presses, the row menu, port links and the
  compose link all look rows up in what the table shows.
- **Kept per table:** tags stay when another list opens; the field's text goes
  away and the list closes. Tags are per resource across engines, like the sort.
- **Compose link:** from a row, Overview or Inspect it opens Containers, clears
  the field's text and sets `compose:<name>`, replacing another `compose` tag and
  keeping other keys. It no longer sends the inspector back to Overview when
  Containers is already open.
- **`show_item`** clears the field's text, and removes the target table's tags
  only if they hide the item. An item not listed yet (a container just run) cannot
  be checked, so its tags go too.
- **Empty message:** "Nothing matches this search." replaces the text that quoted
  the name filter.
- **i18n:** new `docker_search_placeholder`, `docker_tag_remove`,
  `docker_tags_clear_all` and `docker_nothing_matches` in the ten locales, with
  `Key::ALL` updated. Removed `docker_filter_placeholder`,
  `tpl_docker_compose_chip`, `docker_show_all_containers` and
  `tpl_docker_nothing_matches`. Tag keys stay English.

**The age clock.** Age cells depend on the time, and the view and the reducer used
to get it separately. With an `age` tag on, a press could then be looked up among
different rows than were drawn, and the row menu could act on the wrong container.
So `DockerState` keeps `age_clock`, moved by `DockerWidget::reduce` after each
intent, and the view and the reducer both filter with it. The age shown in the cell
still uses the time of the draw, so it can differ from the filter's age until the
next Docker intent.

Values leave out the `—` of an empty cell, so `ports:` suggests only real ports.
`compose:<name>` is a substring match like every tag (Decision 21), so a link to
`web` also shows `web-2`.

Tests were written first and seen failing. Reducer tests cover tags per table,
Clear all, a single cross, the compose link replacing, and links to hidden and
visible items. Pure tests cover the schema per resource, several ports, and AND
and OR over the fixtures. `iced_test` runs the full flow over the real reducer:
`sta` Enter, `run` Enter gives the tag, a cleared field and fewer rows; a row press
after filtering hits the right row.
