# 43 — Kit: tag search model

Status: ready-for-agent
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
