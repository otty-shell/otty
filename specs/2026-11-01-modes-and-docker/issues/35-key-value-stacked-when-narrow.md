# 35 — Key-value list stacks key over value when narrow

Status: ready-for-agent
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
