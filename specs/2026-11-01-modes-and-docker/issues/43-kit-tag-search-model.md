# 43 — Kit: tag search model

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 42
Spec: [../spec.md](../spec.md)

## What

Spec Decision 21. A pure, tested model in `crates/ui/kit/src/components/`, with no
iced view and no knowledge of Docker, so other widgets can reuse it.

- `TagSearch` state: input text, tags (`key`, `value`), highlighted suggestion,
  suggestions open or closed.
- A schema the widget supplies: the keys in column order, the names, and the
  values of a key.
- Parsing: text before the first `:` is a key only if the schema has it.
- Suggestions by the rules of Decision 21, built through one private ranking
  function. For now it keeps items containing the typed text, case-insensitive,
  those starting with it first, the given order otherwise; ticket 46 may replace
  its body with `frizbee`.
- Actions: typing, move up and down, apply (Enter or Tab), close, remove the last
  tag, remove one tag, clear all, add or replace a tag from outside (the compose
  link). The result says what changed, so a widget can refilter.
- Matching helper: given a row's values by key, does it pass the tags (different
  keys all, same key any) and the name text (substring, case-insensitive).
- Duplicate tags are ignored.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Unit tests for every rule in Decision 21, including `redis:7` staying plain
  text, a value containing `:` (`8080:80/tcp`), an unknown key, Tab as Enter, Enter
  with no suggestion, Backspace on an empty field, and OR and AND matching.

## Answer

`crates/ui/kit/src/components/tag_search.rs` holds the model; it has no view and
knows nothing about Docker.

- A widget supplies a `TagSchema`: its keys in column order (`&'static` names)
  and one `SearchRow` per row (the name, plus each cell value by key; a cell with
  several values adds each one).
- `TagSearch` keeps the text, the tags, the highlight and whether the list is
  open. `perform(TagSearchAction, &schema)` does what the user did and returns a
  `TagSearchChange`: `Nothing`, `Suggestions` (the same rows pass) or `Filter`.
- `suggestions(&schema)` follows Decision 21 and marks the highlighted entry.
  `passes` and `passes_tags` are the matching helpers. `set_tag` adds or replaces
  a tag from outside, `clear_input` and `clear_tags` serve links.
- All ranking is in the private `rank`, which ticket 46 can replace. Keys go
  through it too, then return to column order, because Decision 21 lists them so.

Choices the spec leaves open:

- A key is recognized in any case (`STATUS:x`) and takes the schema's spelling.
- A value that already is a tag is not suggested again; duplicates compare
  without case.
- After a value is applied the list stays open and shows the keys, because the
  field is empty and focused.
- A name fills the field and closes the list.
- ↑ or ↓ on a closed list opens it without moving the highlight.
- `key:text` in the field that is not yet a tag filters nothing.

Tests were written first and seen failing (52 against empty bodies). They cover
each rule, including `redis:7` staying plain text, `ports:8080:80/tcp`, an unknown
key, Enter with no suggestion, Backspace on an empty field, and OR and AND
matching. Tab is the same `Apply` action as Enter; ticket 44 binds the key.
