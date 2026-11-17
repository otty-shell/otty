# 31 The dashboard in one centred column

Status: ready-for-agent

## Why

The owner wants the profile dashboard in one column at every width, centred
across the main area and read from top to bottom (spec Decision 12),
instead of the two columns of issue 20.

## What to do

1. The dashboard is always one column: the header, then Usage, "Tokens, last
   30 days", Profile, "Top projects, 30 days", "Where the numbers come
   from". The column is as wide as the room, at most the design system's
   `form` width (720, the kit's `size::FORM`), centred across the main area;
   it starts at the top and the screen scrolls down when it is taller than
   the room, as other screens do (`ScreenArea`). Below `screen-min` (592) it
   keeps the screen rules of issue 20 (sideways scroll).
2. Remove the two-column code and its break (`TWO_COLUMNS_FROM`) with its
   tests; keep the header behaviour of issue 26 (the text moves under the
   name, the buttons stay).
3. The bars and the day bars take the column's width.

## Done when

- Tests, written first, cover: one column at a wide, a medium and the
  smallest width, in the order above; the column centred and at most 720
  wide in a wide main area.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17

Built in `otty-app-agents`, the kit and the binary, tests first.

The dashboard is one column at every width: the header, the notes with
Log in, Usage, "Tokens, last 30 days", Profile, "Top projects, 30 days",
"Where the numbers come from". The column fills the width, at most
`size::FORM` (720), and sits in the middle of a page padded 20 at the
sides, as `FormPage` does. It now starts at the top: the old dashboard
had no height of its own, so `ScreenArea` put a short one in the middle of
a tall main area (the name was at y = 320 in a main area 900 high). Below
`screen-min` the `ScreenArea` rules stay (sideways scroll). Removed:
`responsive`, `TWO_COLUMNS_FROM`, `COLUMN_GAP`, `LEFT_SHARE`,
`RIGHT_SHARE`, the two-column tests and the binary's "two columns" window
test (now "one column"). `profile_column.rs` keeps its name; its docs and
those of the top table no longer say "right column".

Header, asked by the owner during the work: the muted plan looked higher
than the name, as the two texts shared one middle. cosmic-text puts the
letters' height (ascent + descent) in the middle of the line box, so the
baseline lies (ascent − descent) / 2 × size below the middle, whatever the
line height. The kit `fonts::ui_baseline_below_middle(size)` reads this
from the bundled Noto Sans (1069 and −293 of 1000; the three weights
agree, a test checks it). The kit `HeadingRow` got `same_baseline(heading
size, note size)`: on one line the note moves down by the difference
(about 2.7 at 20 and 13), and under the name it is not moved. The
dashboard passes `TITLE` and `BODY`; the gallery entry uses it too.

Tests seen failing first: the dashboard's order at three widths, the
column centred and 720 wide, the more button at the column's end (two
tests), the start at the top; the kit's baseline value and the note on
the heading's baseline (against stubs). Not seen failing: the day bars
filling a main area narrower than a form (the old one column already did),
the binary's one-column window test (not run before the change), the
weights sharing a baseline and the note under the name not moving (both
passed against the stubs), and the dashboard's baseline check of the plan
(it passed against the stubs, which gave 0).

Checks: fmt, clippy, deny and one workspace test run (5628 passed) pass.
Coverage was not measured.
