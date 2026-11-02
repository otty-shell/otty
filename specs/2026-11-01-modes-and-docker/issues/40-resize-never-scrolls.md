# 40 — Dragging a split never scrolls, dragging a scrollbar never resizes

Status: ready-for-agent
Type: bug
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 39
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-02). When a resize drag starts next to a scrollbar, both
happen at once: the split moves and the content scrolls. It must be one or the
other, everywhere:

- A press that starts a resize only resizes, for the whole drag.
- A press on a scrollbar only scrolls, for the whole drag; no resize starts.

"Everywhere" means every split next to scrollable content: the left panel | main
and main | right panel splits (`app/src/widgets/sidebar/view/panels.rs`, kit
`split` style, `SPLIT_GRAB`), the terminal pane grid's splits next to the
terminal's own scrollbar (`crates/ui/terminal`), and every scrollable those
panels hold (Sessions, Launches, Files tree, Docker tree, tables with their
vertical and sideways bars, inspector, Run form, Settings).

Research first and note the answer in `## Comments`: read how iced 0.14's
`pane_grid` and `scrollable` handle a press (the order the content and the split
regions see the event, and when each captures it), and reproduce the double
action in an `iced_test`. Then fix the cause in one place if at all possible, not
per scrollable, for example a kit-level rule where the split's grab zone hides
pointer events from the content under it, or keeping scrollbars out of the grab
zone. Decide who wins where the two zones overlap, and keep the scrollbar
reachable and usable after the fix. Also check that nothing under the pointer
reacts while a resize drag passes over it (hover, scrollbar grab).

The terminal widget may change only if the cause is really there; say why if so.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`, in a pane grid holding a scrollable with its bar next to the split:
  a press and drag in the overlap gives resize messages and no scroll; a press
  and drag on the bar away from the split scrolls and gives no resize; dragging a
  split across scrollable content does not scroll it.
- The same on the root view's real panels and on a terminal pane split, where a
  test can reach it. Otherwise add a step to `regression-checklist.md`.
- The regression checklist gets a step for this.
