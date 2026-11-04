# 36 — Narrow window: tree labels and short table columns stay readable

Status: resolved
Type: bug
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

Seen on the 900 px snapshot of ticket 19 (`12-narrow-overview`), spec
Decision 19 (layout for small screens):

- **Tree rows.** `TreeRow` gives the count the room first, so "Containers"
  with "2 running" is cut to "Co", and the engine root loses its last digit
  ("Docker 29.9"). The label comes first: it keeps its width and ends with
  "…" only when it truly does not fit, and the count or note gives way first
  (it shrinks with "…" or hides when too little room is left). Use the kit's
  `EllipsisText`. Other callers of `TreeRow` (Files, Launches) keep working.
- **Table columns.** All columns shrink by share, so short columns become
  useless: "Age" reads "A…" with values "1…", and ports read "18080:8…". A
  column can state a minimum width (`TableColumn::min_width`) that it never
  goes below; the columns with no minimum give way first. Set minimums in the
  app for short columns (Age, Status, ID, Size, Used by, Driver, Scope,
  Containers) and enough for one port on Ports. When even the minimums do not
  fit, the table scrolls sideways with the thin kit scrollbar, and the header
  scrolls with it, instead of cutting. The row "⋯" column stays reachable by
  scrolling.

Tests first. Gallery updates in both themes. Run fmt, clippy, tests, `cargo
deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Kit `iced_test`: a `TreeRow` at a narrow width keeps its whole label and
  cuts or hides the count; a very narrow one ends the label with "…".
- Kit `iced_test`: a column with a minimum keeps it at a narrow width while
  others shrink; below the sum of minimums the table scrolls sideways and the
  header moves with the rows; row, link and "⋯" presses still hit the right
  row and column after a sideways scroll.
- A snapshot of the 900 px window shows "Containers" and "Age" readable.

## Answer

Tests were written first and seen failing (8 kit tests: the width split,
the minimum kept, the header moving with the rows, and the two tree row
cases). Then the code.

### Kit (`otty-ui-kit`)

- `TreeRow`: the label is an `EllipsisText` that takes the room it needs
  first. The state word and the count are now one text on the right
  ("running 2"), which takes what is left and ends with "…" or shows
  nothing when the room is gone. Other callers are unchanged.
- `TableColumn::min_width(u16)` (pixels, side padding included, default 0).
  A private `column_widths` splits the width by shares; a column below its
  minimum keeps the minimum and the rest share what is left, repeated until
  stable. When the minimums do not fit, columns are their minimums (a column
  with no minimum is 0 wide, so give every column a minimum).
- `DataTable` is built inside `responsive`, so it knows its width. When the
  minimums do not fit, header and rows sit in one horizontal `scrollable`
  (new `scrollbar::horizontal()`, same thin style). New private
  `table_sideways.rs` (`SidewaysScroll`) reads the scroll offset before each
  event; `TableBody` takes it out of right-press and "⋯" points, so the row
  menu still opens at its button. Handlers became `Rc` for this.
- Gallery: a "minimum widths" section (520 and 320 wide) and a "TreeRow
  narrow" section (200, 150, 90 wide), in both themes.

### App

Minimums in `resource_rows::columns()` (px): Containers Name 110, Status
104, Image 96, Compose 80, Ports 130 (one port), Age 52 (sum 572, fits the
576 px a 900 px window leaves); Images 140, ID 118, Size 76, Used by 76, Age
52; Volumes 140, Driver 76, Used by 110, Age 52; Networks Name 120, Driver
76, Scope 72, Subnet 124, Containers 100, Age 52. Tests of the columns
updated.

Coverage (lines): 87.54% (68517 lines, 8536 missed) -> 87.59% (68968 lines,
8559 missed). fmt, clippy, workspace tests and `cargo deny check` pass.

### Notes

- Snapshot of the 900 px window (throwaway, not committed):
  `scratchpad/shots36/narrow-dark-wgpu.png` and `narrow-light-wgpu.png`.
- When the table scrolls sideways the vertical scrollbar sits at the right
  end of the wide content, so it shows after scrolling right.

## Comments

### Forms follow the same rule (owner request, 2026-11-04)

Spec Decision 23. Forms now do what tables do: they shrink to a minimum width
and scroll sideways below it.

- `FormPage::min_width` (default `size::FORM_MIN`, 400). Settings 480, launch
  editor 400, Run container 520, measured with the real fonts in all locales
  (Russian and German are the widest, Korean not counted). The numbers and
  how they were chosen are in comments beside them.
- The tables' `responsive` does not fit: it rebuilds its content per width, and a
  `FormPage` holds elements it was handed once. So the private kit widget
  `FormSheet` lays out the content at the room there is or at the minimum, read
  from the lower limit that the surrounding row and the scrollable pass down. One
  `scrollable` scrolls both ways (`scrollbar::both()`), so both bars stay on the
  edges of the area. In a sideways table the vertical bar sits at the far end of
  the wide content; here it does not.
- The bar of `FormPage::footer_bar` stays pinned and keeps the same minimum. It
  scrolls on its own and is anchored right, so actions on its right need no
  sideways scrolling. (Removed the same day, see the next note.)
- `Dialog` scrolls both ways in a window too small for it. The app's window is at
  least 800 x 600, so today that takes a very tall dialog.
- Limits: a focused input scrolled out of view still takes typing, but the view
  does not jump to it, as before this change for scrolling down. The read-only
  label of a volume row is cut with "…" in Russian and Portuguese at every width,
  because its column is 120 px; not changed here. The app was not launched: see
  the steps 24a in this spec's checklist and 26a in the workspace-layout one.

### A form and its buttons are one screen (owner request, 2026-11-04)

Spec Decision 23. Replaces the pinned bar above.

- `FormPage` now has one footer slot, `footer`, and `footer_bar` is gone, with
  the bar's own scroller and its start at the right end. Settings (Reset, Save),
  the launch editor (Cancel, Save) and Run (hint or pull line, Cancel, Run) all
  use it. One scroller per page holds the fields and the bar.
- `FormSheet` now works for the height too. It first lays out the content to learn
  the height it needs. If that is less than the room, it lays it out again in the
  room, so a `Fill` works: the sections take what is left and the bar lands at the
  bottom. So `FormPage` wraps its scroller in `FormSheet::whole_room`, as
  `EmptyCard` does, instead of a `row`.
- The bar is at least 60 px and grows with a hint or a pull line on several lines.
  In the Run footer the text is `Fill`, so Cancel and Run get their room first; the
  pull line (shared with the Pull dialog) wraps instead of squeezing the bar and
  the amount.
- The Run side panel, the same day: the status badge wraps as in the inspector, and
  the panel's padding moved inside its scroller with the kit's thin bar, because the
  wide default bar covered the right end of the text at 128 px. `StatusBadge` itself
  did not change: it stays on one line by default for table cells.
- Limits: the buttons are no longer in reach without scrolling on a tall form, by
  the owner's choice. The app was not launched: see the steps 24a and 24b in this
  spec's checklist and 26a in the workspace-layout one.

### One container for every screen (owner decision, 2026-11-04)

Spec Decision 23, rewritten. Replaces the separate minimums and `FormSheet` above.

- New kit component `ScreenArea` (`screen_area.rs`), with the one scroller and the
  layout `FormSheet` did. It is chosen in `app/src/view.rs`: `view_tab_content` and
  `view_docker_main` return a `MainContent`, either a `Screen` or `ScrollsOnItsOwn`
  (the terminal and tables), so a new screen can not skip the choice.
- One minimum, `size::SCREEN_MIN`, 592 px. Checked in the ten locales with the kit's
  fonts at that width: every row stays inside, also with the SELinux switch. In
  Korean the SELinux hint still wraps letter by letter, as it did at this width
  before, because Hangul falls back to a wider font.
- Gone: `FormSheet`, `FormPage::min_width`, `EmptyCard::into_scrolling_page`,
  `size::FORM_MIN`, `size::EMPTY_CARD_MIN`, `SETTINGS_MIN_WIDTH`,
  `RUN_FORM_MIN_WIDTH`, and the plain `column`s around the start and searching cards.
- Screens no longer share scrolling. iced hands a widget's state to whatever comes
  next in the same place, and now every screen has the same shape. A `keyed::column`
  does not help: with one child it compares the keys and still hands the state on
  (`iced_widget::keyed::column`, `diff`). So `ScreenArea` keeps its key in its own
  widget state and builds its state tree anew when the key changes. The key is
  `ScreenId`: one per tab, the start card, the Run form, the searching card, one per
  no-engine diagnosis.
- A screen that fits is laid out twice, to learn its height, as before. There is one
  area per screen, so this does not multiply.
- Limits: going back to a screen starts at its top. The app was not launched: see the
  steps 15j, 15k and 24a in this spec's checklist and 26a, 26b in the workspace-layout
  one.
