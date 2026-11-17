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
