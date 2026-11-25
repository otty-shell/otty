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

### 2026-11-16

Built the project screen in `otty-app-agents`, tests first. The title row
has the folder name, its path in mono, "Open the folder in a terminal tab"
(a shell tab in the folder, as Open in terminal of Files) and the New
session `SplitButton`. Under it a `Toolbar` with a `TagSearchField` (keys
Title, Agent, Profile, Branch) and Refresh, the tags row, the `Notice` of a
gone folder, and a `DataTable` sorted by Updated, newest first; text columns
start A to Z, Updated newest first, a second press turns the column around.
The search of each project and the sort live while OTTY runs
(`sessions_table.rs`, `session_sort.rs`). A press selects a row; a double
click or Enter continues it; a right click or the "⋯" opens the row menu at
the pointer: Continue, Continue in a split, Copy resume command, Copy
session ID, Open folder in terminal (`session_actions.rs`). A gone folder
keeps its table, with Continue, Continue in a split, Open folder and New
session disabled. The old session list of issue 05 and the profile choice
in the main area are gone: with more than one profile the plus of New
session opens the same profile menu as the chevron, at the pointer, in the
left panel and in the title row.

Deviations and additions: the Messages column is left out. Claude Code
files are read only at their start and end, and Codex files only at their
first line (most of them compressed), so a count would mean reading every
file whole and would make the session list slow. The kit's `DataTable` had
no double click; it got `on_row_double_press`. The terminal feature could
not run a command in a new split; it got
`TerminalIntent::SplitFocusedPaneWith` and
`TerminalWorkspace::has_local_pane_in_front`, and the binary tells the
feature through `AgentsCtx::has_local_pane`. Continue in a split splits the
focused pane to the right when the tab in front is a terminal tab on this
computer, else it opens a tab; a pane of an SSH tab is never split for a
local agent. The agent in a split pane closes its pane when it ends, as any
program in a shell tab's pane does, so its error does not stay as it does
in an agent tab; the owner keeps this for now, as a known gap. Copy resume command copies the variables of the profile
too (`CODEX_HOME='…' 'codex' 'resume' 'id'`), so the line also works for an
added profile; it is quoted by `otty_app_shell::command_line`, the same
quoting as the tab. Enter that no widget takes reaches the feature as
`AgentsIntent::ContinueSelected`; it does nothing outside the project
screen. The search fields take their own Enter, so it never continues a
session: the kit's `SearchField` got `on_submit`, and Enter in the projects
search of the left panel selects the first project the filtered list shows
(nothing when it shows none); the `TagSearchField` over the table already
took Enter, which a test now checks.
