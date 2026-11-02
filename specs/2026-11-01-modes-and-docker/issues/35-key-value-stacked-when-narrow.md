# 35 — Key-value list stacks key over value when narrow

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 34
Spec: [../spec.md](../spec.md)

## What

Owner request (2026-11-02): in a narrow right panel in Docker mode the
two-column key | value list leaves too little room for values. When the list
is narrow it switches to one column: the key (muted, small) on its own line
and the value under it, with a small gap between entries.

- Kit `KeyValueList` (`crates/ui/kit/src/components/key_value_list.rs`):
  built with `responsive`; two columns at or above a width threshold, stacked
  below it. Pick the threshold so the value column keeps at least about
  160 px in two-column mode (key column 96 + gap); put it in `size.rs` with a
  doc comment. Links, wrapping (`WrappedText`) and value parts (ticket 31)
  work the same in both layouts.
- Gallery: the inspector section shows both layouts.
- The app needs no change unless a caller sets a fixed width that blocks
  `responsive`; check the inspector's Overview.

Tests first. Run fmt, clippy, tests, `cargo deny check` and compare
`cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test`: at a wide width the value sits right of its key; at a
  narrow width the value sits below its key; links still emit their
  messages in both layouts.

## Answer

- `size::KEY_VALUE_STACK_BELOW` = 96 (key) + 8 (gap) + 160 (values) = 264 px.
  Below it each value sits under its key (gap `SPACE_1`); at 264 and wider
  it stays two columns.
- `KeyValueList` is built with `responsive(...).height(Length::Shrink)`, the
  same pattern as `inspector_tabs`. With a shrinking height the list is as
  tall as its entries, also inside the inspector's `scrollable` (checked with
  an `iced_test` that places the list in a scrollable). The app needed no
  change.
- Links and `WrappedText` wrapping are shared by both layouts through one
  private `entry_view`. Two old wrap tests now use narrower widths (160 and
  96 px), because at 250 and 200 px the list is stacked and the value fits.
- Gallery: the inspector section shows the list at 320 px and at 200 px.
- Line coverage 87.20% -> 87.22%.
