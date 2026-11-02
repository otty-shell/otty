# 39 — Inspect controls: switch labels never wrap, copy icon never overlaps

Status: resolved
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

## Answer

The Inspect controls keep every label on one line, and the copy icon never
touches a label. Tests were written first and seen failing (kit: 3 of 5 new
`Switch` tests; app: the layout, the 240 px and minimum width tests, the Follow
tests and the dialog label test), then the code.

### Kit

- `Switch` draws its label with the kit's `EllipsisText` (one line, as wide as
  its text, ends with "…" when the room runs out). This holds everywhere a
  `Switch` is used. Widget queries still see the whole label.
- New `Switch::width()`: the room the switch takes with the whole label, so a
  caller can decide what fits. It measures with the same font and size as the
  drawing.
- Gallery: the Switch section has a long label in a 180 px box.

### App

`view/inspector.rs`: `inspect_controls` is `responsive` and asks a small pure
function, `controls_layout`, what fits in the width it gets:

- All on one row: the switches, then the icon at the end.
- Each switch on its own row with `SPACE_3` between rows, the icon at the end of
  the first row, with at least `SPACE_3` to the label.
- When one switch and the icon do not fit on a line: each switch on its own row
  and the icon on its own line at the end.

The same rule applies to raw JSON, where only one switch shows.

Follow (ticket 22) used the kit `Switch` beside an always present `Fill` box.
That box cost the switch a gap even with no count to show ("Foll…" at the
minimum width) and left a waiting count no room at all (6 px wide, 48 px tall).
Now the count and the switch sit in a wrapping row at the end of the header, and
the count moves above the switch when the two do not fit.

### Long labels elsewhere

The other switch labels can no longer grow to a second line, so a label too wide
for its box now ends with "…":

- The confirm dialog (388 px inside) has to show its label whole. The Russian
  force label did not fit, so it is shorter now: "Принудительно: есть
  остановленный контейнер". A test measures both dialog labels in the English,
  French, German, Spanish, Portuguese and Russian catalogs. The Chinese,
  Japanese and Korean labels are not measured, because a test machine may have
  no font for them; they are short.
- The Run form lives in the main area, up to 720 px wide. Its labels fit unless
  the main area is squeezed very small by wide panels.

### Tests

- Kit (5): a long label in a narrow box stays one line and inside it, a press on
  it still sends the message, `width()` with and without a label.
- `controls_layout` (pure): every boundary, with and without Show empty.
- `iced_test` at 128 px (minimum), 240 px and 420 px, and raw JSON at 240 and
  128 px: each label one line tall and inside the panel, the row gap, the icon
  clear of the label, each control sends its intent.
- Follow at the minimum width: one line, its intent, the whole label when nothing
  waits, the count above the switch when lines wait.
- i18n: the dialog labels fit the dialog.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 87.78% (70246
lines, 8585 missed) on `12e4bf8` -> 87.84% (70638 lines, 8588 missed).
`cargo +nightly fmt`, `cargo clippy --workspace --all-targets --all-features --
-D warnings`, the workspace tests and `cargo deny check` pass.

### Deviations and limits

- The panel's minimum width is 128 px: 16% of an 800 px window. That leaves
  96 px inside the padding, less than the 34 px track, the gap and the text
  need, so at this width "Raw JSON" and "Show empty" end with "…". One line and
  no overlap hold; both whole labels return from about 150 px. The owner's
  screenshot fits a panel of about 170 px (the picture looks scaled by about
  1.75, judging by the 34 px track), where both labels show whole.
- With lines waiting, the count now sits right next to Follow at the end of the
  row, not at the far left of the header.
- A shortened label has no tooltip.
- Snapshots were checked with the bundled fonts in OTTY Dark at 120, 170, 240,
  340 and 420 px (throwaway test, not committed). Not opened in the real app.
