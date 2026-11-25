# 20 Agents mode at any window width

Status: ready-for-agent
Blocked by: 19

## Why

The owner asked for the screens of Agents mode to adapt to the room they
get. Today the dashboard keeps two columns and fixed rows however narrow
the main area is, and nothing has been checked at small widths. OTTY's
design system already has the rules (its README, sections "Screens and
forms" and "Wrapping"); this issue applies them to Agents mode.

## The rules to apply

From the OTTY design system:

- A screen that is not a terminal or a table lives in one `ScreenArea`: it
  takes the room's width down to `screen-min` (592); in less room it keeps
  that width and scrolls sideways. A taller screen scrolls down. One
  scroller; thin bars that float over the content.
- Text in a panel never overlaps and never pushes a control out: the main
  text keeps its room first and ends with "…" only when it truly does not
  fit; notes and counts give way first. Action buttons keep their size.
- A `DataTable` column keeps its minimum; when the minimums do not fit,
  header and rows scroll sideways together.
- A `Toolbar` field shrinks first, then the last buttons move into a "⋯"
  menu.
- `KeyValueList` values wrap after a space, `/`, `-`, `:` or `.`; below
  `key-value-stack-below` (264) each value sits under its key.
- Dialogs keep their width in a window too small for them and scroll both
  ways.

## What to do

1. **Profile dashboard** (`ScreenArea`): two columns while the main area is
   at least about 900 wide (pick the exact break from the content and say
   it); below it one column in the order: billing block, tokens, Profile,
   top projects / by model, sources. The header row wraps: the title, badge
   and plan stay on the first line, the buttons keep their size and go to
   a second line when they do not fit, never over the text. The three
   totals of a block (today, 7 days, month) wrap to the next line instead
   of being cut. Bars and the day bars take the column's width.
2. **Project screen**: the title row keeps the two icon buttons at the
   right; the folder name keeps its room, the path gives way first and ends
   with "…". The toolbar follows the `Toolbar` rule. The sessions table
   follows the `DataTable` rule (it already scrolls on its own; check the
   minimums make sense: Title wide, the rest narrow).
3. **Left panel**: check every row at the panel's minimum width (16 % of
   the window, 128 at the least): Agents rows (logo, name, number), project
   rows, the search field with its funnel, the chips (they wrap), "Hidden
   profiles (N)". Nothing overlaps; the right-hand text gives way first.
4. **Dialogs and menus**: the profile dialog and the confirmation dialog in
   a window smaller than them scroll; menus opened near a window edge stay
   inside the window.
5. **Status bar**: the left side gives way before the right side.

## Done when

- Tests, written first, render each screen with the iced test simulator at
  a wide size (1280), a medium one (about 800 of main area) and the
  smallest (the panel at its minimum, the main area at 592 or less), and
  check: one or two columns as above, nothing cut that must stay (the title,
  buttons), the sideways scroll where the rules say.
- Run by hand: the steps the agent writes in its report (resize the window
  from wide to narrow on each screen).
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built in `otty-app-agents`, the kit and the binary, tests first. The tests
of the dashboard, the totals and the status bar were seen failing first;
the tests of the project screen, the left panel at 128, the dialogs and
the menus passed on the code as it was, so those places already followed
the rules and only got tests (and the folder name its "…").

Profile dashboard: the columns are built in `responsive` and split at
`TWO_COLUMNS_FROM` = 860 of room for the columns, a main area of 900. There
the right column gets (860 − 32) × 2/5 = 331, enough for the minimum
columns of the By model table (120 + 3 × 64 = 312), so in two columns that
table never scrolls sideways. Below it one column: the notes, the billing
block, the tokens, Profile, the top table, the sources. The header is a new
kit `HeadingRow` (gallery entry and tests): the logo, the name, the badge
and the plan on the first line, the buttons at the right end while they
fit, else on a line of their own under the name, at their size. The name
and the plan are `EllipsisText`s; the plan gives way first. The totals
row wraps (`Row::wrap`), a total goes whole to the next line. The bars and
the day bars already took the column's width; a test checks it.

Project screen: the folder name is an `EllipsisText` that takes its room
first, so it ends with "…" only when it alone does not fit; the path gives
way first and the two buttons stay at the right. The toolbar and the table
were already right; the minimums (Title 220, the rest 96–120) sum to 656,
so the table fits the main area of the smallest window (about 670) and
scrolls sideways in a main area of 592.

Left panel at 128: the rows, the header buttons, the field with the funnel,
the chips (they wrap) and "Hidden profiles (N)" do not overlap; nothing
changed. Dialogs and menus: the kit `Dialog` already scrolls both ways and
`context_menu` keeps a menu inside the area; tests check both in Agents
mode. Status bar: the binary builds it in `responsive` and leaves out the
last left texts, whole, until the right side fits, as the frame's bar does
with the block counter.

Deviations: no main area of 592 or less can come from the smallest window
with the panel at its minimum (the main area is then about 670), so the
592 cases are tested on the screens themselves, and the window tests use
1280 × 800 and 800 × 600.
