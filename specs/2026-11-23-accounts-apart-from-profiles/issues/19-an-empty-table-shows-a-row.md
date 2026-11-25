# 19 An empty table shows its message as a row, and tables keep their bottom padding

Status: done
Blocked by: 18

## Why

Owner, 2026-11-25: when a table says "Nothing to show" it looks odd, as
if there were no row, only empty space, and the tables lack their bottom
padding.

Likely cause, to confirm with a picture: the kit `DataTable` draws its
empty message (`empty_view` in `crates/ui/kit/src/components/data_table.rs`)
as muted text with `EMPTY_PADDING` (16 px over and under), not as a row,
so it does not look like part of the table; and the usage block sizes its
table to its rows (`blocks::rows_tall(table, rows)` with at least one row
in `crates/app/features/agents/core/src/view/usage_block.rs`), so the
taller empty message is cut at the bottom.

## What to build

1. **The empty message is a row.** A table with no rows shows its message
   in one row of the table: the row height, the cell padding at the left,
   the text muted and centred up and down, and the row's bottom line as
   the other rows have, so the table reads as a table with one row that
   says why it is empty. Every table of the app with an empty message
   gets it (Agents usage tables and popups, the sessions table, All
   accounts and All profiles, Containers lists, Plugins, and any other
   `empty_message` user).
2. **Bottom padding.** Check every table placed in a block or card
   (usage block, statistics popups, account and profile pages,
   Containers) at the bottom of its block: the space under the last row
   (or under the empty row) is the block's own padding, as on its other
   sides. Fix where a table is cut or sits flush on the block's edge,
   including any height that is computed from the number of rows.
3. Check by pictures (kit and app tests draw screens; save a few to look
   at while working, as earlier issues did) and pin the geometry in tests.

## Done when

- Tests first: an empty table's message sits in a row as tall as a row
  with the bottom line, the text at the cell padding and centred up and
  down; the usage block with no rows shows the whole empty row and its
  block's bottom padding; a table with rows keeps the block's bottom
  padding.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-25: the pictures showed the cause. The kit `DataTable` drew its
empty message as muted text with 16 px over and under it, not as a row,
and the usage block gives its table the height of one row, so the
message was squeezed and cut at the bottom of the table. Now the empty
message is one row of the table: the row height, the cell padding at the
left, the text muted like a muted cell and centred up and down, and the
row's bottom line. It fits the height of one row exactly, so the usage
block on a profile page and in the statistics popups shows the whole row
and keeps the room under it as with rows. The tables with no empty
message (All accounts, All profiles, the found report) and the tables
that fill their area (Containers, the sessions table) needed no other
change. Tests pin the place of the message in its row, an empty row that
draws the same as a blank row, and the room under the usage table with
and without rows, in the popup and on the profile page.
