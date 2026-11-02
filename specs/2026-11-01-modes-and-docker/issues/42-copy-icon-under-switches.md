# 42 — Narrow Inspect: the copy icon sits on the left under the switches

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 41
Spec: [../spec.md](../spec.md)

## What

Owner request (2026-11-02), changes ticket 39's layout. When the Inspect controls
do not fit on one row:

- the switches stack, each on its own row;
- the copy icon goes on its own row right under them, aligned to the left edge,
  with the same gap as between the switches.

So there are only two layouts now: everything on one row (icon at the end, as
today), or the stack above. The layout where the icon stays at the end of the
first switch's row goes away; simplify `controls_layout` accordingly. The icon's
left edge lines up with the switches' left edge.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo
llvm-cov` with the merge base before the commit.

## Done when

- Pure test of `controls_layout`: wide gives one row, narrow gives the stack.
- `iced_test` at 128, 170, 240 and 420 px: below the switches at the narrow
  widths with the same left edge (compare bounds), at the end of the row at 420;
  the icon still emits the copy intent.

## Answer

The Inspect controls now have two layouts only. Everything fits: one row, icon
at the end. Otherwise: the switches stacked, and the copy icon on its own row
under them at the left edge, with `SPACE_3` between rows like the switches.
The layout with the icon at the end of the first switch's row is gone;
`ControlsLayout` is `OneRow` or `Stacked`, and `controls_layout` is one
comparison (all controls on a line, or not).

Tests were written first and seen failing. Pure test: wide gives one row,
narrow gives the stack. App tests at 128, 170 and 240 px click the icon under
the switches at the left edge (and check the switch rows, gap and one-line
labels); at 420 px the icon is at the end of the row; the icon still sends the
copy intent. Raw JSON alone follows the same rule.
