# 39 The Containers list shrinks like every screen; a plugin page starts at the top

Status: ready
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
