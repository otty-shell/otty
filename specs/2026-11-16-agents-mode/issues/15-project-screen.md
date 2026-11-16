# 15 The project screen: the sessions table

Status: ready-for-agent
Blocked by: 14

## Why

The owner wants the sessions of a project as a table, like the lists of
Containers mode (spec Decision 6, canvas screen "Project: sessions table").

## What to do

1. The title row: folder name, its path in mono and muted, and on the right
   two icon buttons: "Open the folder in a terminal tab" (a tab with the
   user's shell in that folder, as Open in terminal in Files does) and the
   "New session" `SplitButton`.
2. A toolbar with a `TagSearchField` (keys Title, Agent, Profile, Branch)
   and Refresh, built the way Containers mode builds its toolbar and tag
   search; the tags and the typed text of each project live while OTTY
   runs.
3. A `DataTable`: Title, Agent, Profile, Branch, Messages, Updated, with the
   column shares and minimums of Containers mode's tables. Default sort:
   Updated, newest first; every column sorts as the design system's table
   rules say. The number of messages comes from the session reading (count
   the user and assistant records; add it to `AgentSession` where it is
   cheap, else leave the column out and say so).
4. Enter or a double click continues the selected session. The "⋯" of a row
   and a right click open the menu: Continue, Continue in a split, Copy
   resume command, Copy session ID, Open folder in terminal. "Continue in a
   split" splits the focused pane of the active terminal tab when there is
   one, else opens a tab.
5. A project whose folder is gone shows the table with Continue and New
   session disabled and a `Notice` that says why.
6. Remove the issue 05 session list this replaces.

## Done when

- Tests, written first, cover the reducer: sort, tags, Enter, the row menu
  actions and the commands they report, a missing folder.
- Run by hand: the steps the agent writes in its report.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16 (coordinator)

From issue 14: with more than one profile, the plus of "New session" (in the
left panel and in the title row here) opens the same profile menu as the
chevron, at the button, instead of the choice shown in the main area.
Remove that main-area choice. A window used 80 % or more is drawn in the
waiting color in the left panel; keep that.
