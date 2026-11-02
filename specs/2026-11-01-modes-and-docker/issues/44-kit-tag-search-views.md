# 44 — Kit: `TagSearchField` and `TagRow` views

Status: ready-for-agent
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
