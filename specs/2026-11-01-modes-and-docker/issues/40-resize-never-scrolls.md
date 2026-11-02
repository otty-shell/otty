# 40 — Dragging a split never scrolls, dragging a scrollbar never resizes

Status: resolved
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

## Comments

### How iced 0.14 orders a press (researched 2026-11-02)

Read in `iced_widget-0.14.0/src/pane_grid.rs`, `pane_grid/axis.rs` and
`scrollable.rs`.

- `PaneGrid::update` first passes every event to the content of **every pane**,
  with the real cursor (lines 509-528). Only then does it look at the event
  itself (line 530). It never checks whether the content captured the event.
- `Scrollable::update` grabs its scroller on a press over its bar and scrolls on
  each move while grabbed. It captures the event, but nothing above it listens.
- So on a press inside a split's zone both start: the scrollable grabs its bar
  and `PaneGrid` sets `Action::Resizing`. Each move then scrolls and sends a
  resize. There is no way to tell `PaneGrid` to stop forwarding, and it has no
  way to say "the content wants this press".
- The zone is `split_line_bounds(region, ratio, spacing + leeway)`, centered on
  the line. With `spacing(1)` and `on_resize(8, ..)` that is the line and 4 px on
  each side. Thin kit bars are 4 px wide on the right edge of a pane. So a bar
  right beside a split lies **entirely** inside the zone: with the split on top
  and no other change, that bar could not be reached at all.
- A hidden cursor already exists in iced: `Cursor::levitate()`. For a levitating
  cursor `position()` is `None`, so widgets act as if the pointer were elsewhere.
  They still get the events, so a text field still loses focus on a press.
- The terminal (`crates/ui/terminal`) has **no scrollbar**, only the wheel. Its
  double action is a different one, with the same cause: a press in the zone over
  a terminal also starts a text selection (or a mouse report to the program) that
  goes on during the drag. When the release lands outside the terminal, the
  terminal never sees it. The cause is in the grid, so the terminal crate does
  not change.

The double action, reproduced in `iced_test` on `PaneGrid::on_resize` before the
fix. Messages of one press and drag on a bar beside a split:

```
only a scroll should happen: [Scrolled, Scrolled, Resized, Scrolled, Resized]
```

The same with a press in the overlap at the end of a bar, and a sensor under a
resize drag reported the pointer moving over it.

### Who wins

**The split wins wherever the zones overlap**, as the "Done when" asks, and the
zone leaves room for the bars. Kit bars only ever sit on the right and the bottom
edge of what they scroll. So the zone is now the line and 4 px to its **right**
(or **below** it), and nothing on the other side:

- A bar right beside a line is outside the zone, all 4 px of it, so it stays
  fully usable.
- The only overlap left is the first 4 px of a bar that starts at a split (the
  vertical bar of the lower pane, the sideways bar of the right pane). There the
  press resizes, because the rest of the bar is still reachable and the split has
  no other place to be grabbed.
- A terminal left of a split gets its last 4 px back, so selecting text there
  works. Right of a split nothing changes: the first 4 px still belong to the
  split.

The zone gets narrower: 5 px instead of 9. Making it wider to the right would
cost a terminal's first column, so it stays at that.

## Answer

A press that starts on a split only resizes, a press on a scrollbar only scrolls,
each for the whole drag, in every pane grid. The tests were written first and
seen failing on `PaneGrid::on_resize` (7 of the 15 kit tests, the bar test on the
root view, the terminal test left of the line), then the code.

### The fix, in one place

`crates/ui/kit/src/components/split.rs`: new `ResizableGrid`, a small widget around
a `PaneGrid`. The grid no longer gets `.on_resize`, so its own drag is off and
the wrapper owns it. It remembers who the press belongs to:

- A press in a grab zone: this split. Until the release the panes see a hidden
  (levitating) cursor. They still get the events, so a text field still loses
  focus, but a bar is not grabbed, a button is not pressed, nothing hovers and a
  terminal starts no selection.
- A press anywhere else: the panes. Their drag gets the real cursor until the
  release, even over a split, and no resize starts. A scrollbar drag goes on, and
  a text selection can end on the line.
- No press, pointer over a zone: the panes see it hidden too, so nothing lights
  up there. The wheel and the other buttons still reach the pane, so scrolling and
  the terminal's context menu work next to a line as before.

The wrapper also draws the line under the pointer, sets the resize pointer shape
and asks for a redraw when they change, which `PaneGrid` did before.
`size::SPLIT_GRAB` is now 4 px and one-sided; see Comments for why.

`app/src/widgets/sidebar/view/panels.rs` and
`app/src/widgets/terminal_workspace/view/pane_grid.rs` use the wrapper. No
scrollable changed. In focus mode the panels keep the wrapper without a resize
message, so the widget tree keeps its shape and the main area keeps its state.
The terminal keeps iced's default smallest pane, now written as `MIN_PANE_SIZE`.

`crates/ui/terminal` did not change: the cause is in the grid, see Comments.

Gallery: a `ResizableGrid` section with a bar beside the line.

The abstraction is needed because `PaneGrid` cannot be told who gets a press:
that decision has to sit outside it, and two grids need the same one.

### Tests

- Kit (18, `iced_test`, in a pane grid holding a scrollable):
  - a press on the line beside a bar only resizes, a press on the bar right up to
    the line only scrolls;
  - where a vertical or a sideways bar starts at a split: only a resize; on that
    bar away from the split: only a scroll;
  - a resize passing over a bar does not scroll, a resize passing over a sensor
    and a button sends nothing to them, also when let go on the button;
  - a press in the zone does not reach the button under it, one right of the zone
    does; hovering the zone hides the pointer; the wheel and a right press still
    work;
  - a scrollbar drag over a split keeps scrolling and starts no resize, a content
    drag through the zone keeps reaching the content;
  - without a resize message the zone belongs to the panes; the zone follows the
    grid's place in the window; the ratio under the pointer is right.
- Root view, real panels in Docker mode with a table that scrolls (4): the split
  beside the bar only resizes, the bar beside the split only scrolls, a split
  dragged across the table does not scroll it, focus mode cannot be dragged.
- Terminal pane grid (3): a press on the line and right of it resizes, left of the
  line and away from it does not. The panes in these tests hold no real terminal,
  since one needs a running shell; the regression checklist covers that.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 87.84% (70638
lines, 8588 missed) after ticket 39 -> 88.00% (71373 lines, 8567 missed).
`cargo +nightly fmt`, `cargo clippy --workspace --all-targets --all-features --
-D warnings`, the workspace tests and `cargo deny check` pass.

### Deviations and limits

- A bar right beside a line no longer overlaps the zone at all. The "press in the
  overlap gives a resize" test runs where a bar starts at a split, the only
  overlap left, and on the line beside the bar.
- The zone is narrower and one-sided: 5 px instead of 9. A press up to 4 px left
  of a line, with no bar there, now goes to the pane: a small change for a terminal
  left of a split, whose last 4 px select text now instead of resizing.
- A closed right panel's edge sits at the window edge, so its zone is the last
  pixel. Outside macOS the window's resize grip covers that edge anyway, before
  and after this change. The panel buttons still open it.
- The pointer shape and the lit line are not tested: `iced_test` shows neither.
  Steps 23 and 24 of the regression checklist cover them, with a real terminal.
  Not opened in the real app.
