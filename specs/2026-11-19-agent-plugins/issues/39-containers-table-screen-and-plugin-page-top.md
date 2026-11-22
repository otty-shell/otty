# 39 The Containers list shrinks like every screen; a plugin page starts at the top

Status: done
Blocked by: 38

## Why

Owner, 2026-11-22:

- After issue 38 the Agents project page keeps the screens' minimum width
  and scrolls sideways below it (`MainContent::TableScreen`,
  `ScreenArea::filling_height`). The Containers list (its table) is still
  `MainContent::ScrollsOnItsOwn`
  (`crates/app/features/containers/core/src/lib.rs` `main_area`), so its
  header squeezes with no limit. It should work the same way.
- In Plugins mode the page of a plugin is centred up and down in the main
  area; it should start at the top and run down, as the pages of Agents
  mode do. `ScreenArea` centres a screen that does not fill the room
  ("a screen that does not fill the room, such as a card, is centered
  both ways"); a page is not a card.

## What to build

1. **Containers list as a table screen.** The Containers list uses
   `MainContent::TableScreen` with its own screen key, as the Agents
   project page does: the screen keeps `size::SCREEN_MIN` and scrolls
   sideways below it, the table fills the rest of the height and scrolls
   down on its own. One scroller per direction: if the table's column
   minimums do not fit the smallest screen, the table must not scroll
   sideways inside a page that also does (see how issue 38 handled the
   Title column, and report what was changed). Every other place that
   still uses `ScrollsOnItsOwn` for a page with a header over a table is
   checked the same way (grep `ScrollsOnItsOwn`); terminals stay as they
   are.
2. **A plugin page starts at the top.** The page of a plugin
   (`crates/app/features/plugins/core/src/view/details.rs`
   `plugin_page`) is laid out from the top, centred left and right as
   today, as the Agents pages are. Do it the way Agents mode's pages do
   it (find how they avoid the centring). The "No plugin selected" card
   stays centred both ways.

## Done when

- Tests first: the Containers list is a `TableScreen`, keeps the minimum
  width in a narrow window and its table fits the smallest screen; a
  plugin page's header is at the top of the main area in a tall window;
  the hint card stays centred.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-22, done. The open list of Containers mode is a screen now:
`MainContent::TableScreen` with the new screen key
`ContainersScreen::List(Place)`, one per list of an engine, so another
list starts at the top left and the same list keeps its scrolling. It
keeps `size::SCREEN_MIN` and scrolls sideways below it, and its table
fills the rest of the height and scrolls down on its own. The table has
no side padding, so a list and its "⋯" column (36 px) must fit 592 px.
The minimums of Images (534), Volumes (378) and Networks (544) did; those
of Containers (572) did not, so the minimum of the Name column went from
110 to 94 px, as the Title column of the Agents sessions table went down
in issue 38: the name has the widest share and ends with "…". No table
scrolls sideways itself now, so nothing scrolls twice.

`ScrollsOnItsOwn` is now used by the terminal panes only; its doc says
so. No other page with a header over a table used it.

The page of a plugin fills the height of the main area, as the pages of
Agents mode do (`.height(Length::Fill)` on the page container), so
`ScreenArea` no longer centres it up and down and it starts at the top;
it stays centred left and right. The "No plugin selected" card is still
centred both ways.

Tests: the open list is a `TableScreen` named by its page and keeps the
minimum width at 320 px, and every list fits the smallest screen; a
plugin page's header is at the top of a tall main area, and the hint
card stays in the middle. No user docs described these layouts.
