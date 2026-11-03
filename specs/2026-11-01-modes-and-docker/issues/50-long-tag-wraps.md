# 50 — A long tag wraps and its chip grows

Status: ready-for-agent
Type: bug
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 49
Spec: [../spec.md](../spec.md)

## What

Owner report (2026-11-03). In the tags row of the search, a tag with a long value is
cut. It must wrap instead, and the chip must grow in height to hold every line:

- the whole text of a tag is always readable, however narrow the panel;
- lines break by the kit's wrapping rules (`WrappedText`: after a space, `/`, `-`,
  `:` or `.` first, between letters only as a last resort);
- a chip is never wider than the tags row; short tags stay one line and keep today's
  look;
- the clear cross stays visible, aligned with the first line, and keeps working;
- other chips in the row and "Clear all" stay in their wrapping flow, and the table
  moves down by the height the row takes.

Other uses of `Chip` must not change: make this a choice of the chip or of `TagRow`.

Tests first, gallery update in both themes. Run fmt, clippy, tests, `cargo deny check`
and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test`: a very long value at a narrow width gives a taller chip no wider
  than the row, with the text fully inside and not cut; a short tag stays one line
  tall; the cross of the long tag still emits its message and does not overlap the
  text; other `Chip` uses keep their height.
