# 44 — Kit: `TagSearchField` and `TagRow` views

Status: resolved
Type: task
Class: A
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 43
Spec: [../spec.md](../spec.md)

## What

Spec Decision 21. Kit views over the model of ticket 43, each with a gallery
section in both themes and `iced_test`s.

- `TagSearchField`: a text input with a floating suggestion list under it (the
  highlighted row marked, kept in view while moving, at most about 8 rows then
  scrolling, keys and names told apart, the matched part shown as in the command
  palette). Reuse the private `InputKeys` for ↑ ↓ Enter Tab Esc Backspace, and
  find out how to show the list on focus of an empty field. It must work as the
  field of the kit `Toolbar`.
- `TagRow`: a wrapping row of `Chip`s with their clear cross and a small "Clear
  all" button at the end. It draws nothing and takes no height with no tags.
- Text comes from the caller.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: typing and each key emit their messages; pressing a suggestion
  picks it; the list sits above the content under it and closes on a press
  outside; chips wrap at a narrow width; a chip's cross and "Clear all" emit their
  messages; an empty `TagRow` has no height.

## Answer

Two public views and one private widget, all sending `TagSearchAction`, so a
widget forwards one message to `TagSearch::perform`.

- `TagSearchField` (`tag_search_field.rs`): a boxed input, the same size as
  `TextField`, and the floating list. ↑ ↓ Enter Tab always send their action
  while the input is focused, Esc only while the list is open (otherwise the
  input drops the focus as usual), Backspace only in an empty field. It works as
  the `Toolbar` field; that case has its own test.
- `TagRow` (`tag_row.rs`): a wrapping row of `Chip`s reading `key:value`, each
  with its cross, and a small ghost "Clear all" button. With no tags it is an
  empty `Space`. The texts come from the caller.
- `FloatingList` (`floating_list.rs`, private): the field plus an overlay list.

**Focus on an empty field.** `iced` 0.14 text inputs send nothing on focus, so
`InputKeys` gained `on_focus`. It remembers in its widget state whether the input
was focused after the last event and sends the message when that turns on. A
focus given by the app through an operation is reported on the next event.

**Keeping the highlighted row in view.** The overlay runs a small private
`Operation` on its own list after layout, only when the highlighted row changed,
so no app needs a scroll task and the mouse wheel stays free. The row height is
`size::ROW` and the list shows 8 rows, then scrolls.

**Press rules.** A press in the list is captured, so the input keeps the focus
and typing goes on after a click. A press in the input does not close the list. A
press anywhere else sends `Close` and also reaches what is there, so one click on
a table row closes the list and presses the row.

Keys read differently from names and values: accent color and a trailing `:`.
The matched part is `text-strong` in the bold mono font, using the command
palette's `title_pieces`. No kind label is shown, so the row needs no translated
text.

The gallery has a live section (one seeded tag, twelve rows, so the list
scrolls); the theme button shows it in both themes.

Tests were written first and seen failing. `iced_test` covers typing, each key,
focus reported once, a press picking its position, the input keeping the focus,
the list over the content, a press outside, a press in the input, eight rows then
scrolling, the highlighted row in view, a press on a scrolled row, the disabled
field and the `Toolbar` case; for `TagRow`, wrapping, the cross, "Clear all", no
height when empty and no handler. Pure tests cover the list position and the
scroll offset.

One change outside the ticket: the app's tests read the real
`~/.config/otty/settings.json` when they built `SettingsWidget`, which sets the
process-wide language. With that file set to `ru`, about 30 English-text tests
failed at random, in `cargo llvm-cov` and in the commit hook alike. In test
builds `settings_path` now points to a temporary folder, with a test of its own.
