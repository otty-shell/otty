# 26 — Docker tables: sorting, defaults and kept choice

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 24, 25
Spec: [../spec.md](../spec.md)

## What

Spec Decision 19, second point. Tests first.

- Sort state per table (resource kind): column and direction, kept in the
  widget state while OTTY runs (not saved to disk).
- Defaults: containers running first (then restarting, paused, created,
  exited, dead), then by name; images, volumes, networks by name.
- Header press: a new column sorts in its natural first direction (text A→Z,
  ages and dates newest first, sizes and counts largest first, status by the
  state order above); pressing the same column again reverses it.
- Sorting is a pure, tested function per resource over the shown rows; the
  filter and compose filter still apply; selection stays on the same item
  (by id) after a sort.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure sort tests per resource and column, both directions, ties broken by
  name; default orders.
- Reducer tests: header press sets and reverses; selection kept by id.
- `iced_test`: header press emits the sort intent; the arrow shows.
