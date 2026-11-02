# 39 — Inspect controls: switch labels never wrap, copy icon never overlaps

Status: ready-for-agent
Type: bug
Class: A
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 37
Spec: [../spec.md](../spec.md)

## What

Owner report with a screenshot (2026-11-02). In the narrow right panel the
Inspect controls break: "Raw JSON" and "Show empty" wrap to two lines each, the
two switches almost touch vertically, and the copy icon runs into the "Raw JSON"
label.

- A switch label stays on one line, in the kit and everywhere it is used.
- The controls use the room like this: when both switches fit next to each
  other and the icon, one row; otherwise each switch has its own row with a
  clear gap between the rows (at least `SPACE_3`).
- The copy icon keeps its place at the end of the first row and always has a
  gap to the label; it never overlaps it. If even one switch and the icon do
  not fit on a line, the icon gets its own line.
- The panel's minimum width is the worst case to check.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test` at the panel's minimum width, at about 240 px and at 420 px: each
  label is one line tall, labels and the icon do not overlap (compare their
  bounds), rows have the gap, and each control still emits its intent.
- A snapshot at the width of the owner's screenshot looks right.
