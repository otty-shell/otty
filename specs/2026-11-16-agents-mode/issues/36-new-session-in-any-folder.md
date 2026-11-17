# 36 A new session in any folder

Status: ready-for-agent
Blocked by: 35

## Why

Today a session starts only in a project Agents mode already lists, that
is a folder where an agent has run before. The owner wants to start one in
any folder (spec Decision 15, 2026-11-17), both from Agents mode and from
where they work: a terminal tab or the Files panel.

## What to do

1. **One "New session" dialog** (the kit `Dialog`) in the agents feature:
   - a folder field with suggestions, as the config folder field of the
     profile dialog (reuse it);
   - the profile, a select of the profiles that can start (as the New
     session menu lists them);
   - Cancel and Start.
   - Start is off while the folder does not exist or no profile is chosen;
     the field says why.
   - Start opens the agent tab in that folder exactly as New session on a
     project screen does (same command, same config folder variable).
   - The folder is not stored anywhere. The project shows in the list
     when the agent has written its first session and the projects are
     read again. Read the projects again when the tab opens and when it
     closes, if that is cheap; say what you did.
2. **From Agents mode:** a "+" icon button "New session" in the header of
   the Projects section of the left panel, next to the search button.
   - The folder is the folder of the focused terminal tab when it runs on
     this computer (the folder Files follows, issue 21), else the home
     folder.
   - The profile is the selected profile, else the first one that can
     start.
3. **From a terminal tab:** a command palette command "New agent session
   in this folder". It opens Agents mode with the dialog, the folder set
   to the focused tab's folder. The command shows only when that tab runs
   on this computer and has a folder.
4. **From Files:** an item "New agent session here" in the context menu of
   a folder (and of the empty space, for the shown folder). It opens the
   same dialog with that folder.
5. The dialog keeps the last profile chosen while OTTY runs.
6. All ten locales. Tests of the binary for the palette command, the Files
   item and the left panel button through the whole window (the guard
   allow-list, issue 26).

## Done when

- Tests, written first and seen failing, cover: the dialog (folder
  suggestions, Start off for a missing folder, Start opens the agent tab in
  the folder with the chosen profile); the three ways to open it and the
  folder each gives; the palette command hidden for an SSH tab.
- Run by hand: start a Claude Code session in a new folder from each of
  the three places; after the first answer, the folder shows in Projects.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17 — built

**What was built**

- **The New session dialog** (kit `Dialog`), a new module
  `new_session_dialog/` of the agents feature: a folder field, a profile
  select (kit `select`, the profiles that can start, as the New session
  menu lists them), Cancel and Start.
  - Typing in the field asks `otty-app-agents-io` to look the folder up
    (`AgentsOutcome::LookUpFolder`, answered by
    `AgentsIntent::FolderLookedUp`): whether it is a folder, and the
    folders it may stand for. `~/pro` lists the folders of `~` whose name
    starts with `pro` (case aside, hidden ones only when the dot is typed,
    at most 50); a path that ends with `/` lists the folders in it. The
    suggestions float under the field; a pick puts `~/projects/` in the
    field and lists the next level. An answer for an older text is
    dropped.
  - The field says why Start is off: no path, not a full path, or "There
    is no such folder". While the answer has not come, Start is off with
    no error. With no profile that can start, a note under the select says
    so.
  - Start opens the agent tab through the same `open_tab` as New session
    of the project screen (same command, same `CLAUDE_CONFIG_DIR` /
    `CODEX_HOME`), titled with the profile name.
  - The proposed profile: the one picked last (kept while OTTY runs, also
    after a Start), else the selected profile, else the first that can
    start; each only while it can start.
- **Projects header**: the "+" icon button "New session" next to the
  search button opens the dialog on the folder of the focused terminal
  tab when it runs on this computer (`TerminalWorkspace::focused_local_folder`,
  the folder Files follows, passed in `AgentsCtx::local_folder`), else the
  home folder.
- **Palette**: `AppCommand::NewAgentSessionHere`, "New agent session in
  this folder". It shows only in Terminal mode while the tab in front runs
  on this computer and its folder is known (`CommandCtx::has_local_folder`).
  It switches to Agents mode and opens the dialog on that folder.
- **Files**: "New agent session here" in the menu of a folder and of the
  empty area (the shown folder): `ExplorerMenuAction::NewAgentSession` →
  `ExplorerOutcome::NewAgentSession { folder }`; the binary switches to
  Agents mode and opens the dialog there.
- **Kit**: `TextField::suggestions(list, on_pick)` and
  `on_suggestions_close`: a list floating under the input (the
  `FloatingList` the tag search uses), in the look of a menu.
- **Guards**: `Overlay::AgentsNewSessionDialog`, a dialog; its answers are
  every `AgentsIntent::NewSessionDialog(_)`; Esc cancels it. The look-up
  answer arrives on its own.
- Eight new texts in all ten locales.

**Reading the projects again.** The folder is not stored. Reading at tab
open is useless (the agent writes its first session only after the first
answer) and the feature can not see a tab close. So Start marks the
profile, and the next time Agents mode opens its sessions are read again
(one `LoadSessions` for that profile only, not a full refresh). A full read
of the profiles clears the mark. Refresh still works at any time.

**Tests seen failing first** (each failed to compile, or with a wrong
value, before the code): the kit suggestions (press on one, press away,
none shown); `focused_local_folder` (local and SSH); the command rule;
the palette list; the Files menu items and outcomes; the dialog state
(queries, checking/found/missing, stale answers, suggestions, pick), its
reducer (open on a given folder, on the terminal folder, on home; typing
looks up; Start opens the tab with the chosen profile and the config
folder variable; Start off for a missing or unchecked folder and with no
profile; Cancel; proposed and kept profile; next opening reads the
sessions of the profile), its view model and view; the io look-up on a
temp folder. In the binary, through the window and the input checks:
the "+" press opens the dialog and Start in the window opens the agent
tab in the home folder (mock terminals, no agent runs); typing in the
dialog field; the palette command opens the dialog on the tab folder and
is not offered for an SSH tab; "New agent session here" pressed in the
Files menu in the running window; the guard lists (overlay kind, Esc, own
answers, other dialogs' answers).

**Choices and deviations**

- The profile dialog's config folder field had no suggestions, so there
  was nothing to reuse but its path parsing (`full_path`, `~`); the
  suggestions are new, in the kit `TextField`. The profile dialog itself
  did not get suggestions (left for later if wanted).
- The "+" **replaces** the `SplitButton` "New session" of the Projects
  header: two plus buttons named New session side by side would be
  confusing, and New session of the selected project stays on its title
  row (with its chevron).
- The palette command is limited to Terminal mode, like the other
  commands about the tab in front.
- The dialog folder field takes the keyboard when the dialog opens.

**Check by hand**

1. Terminal mode, `cd` to a new folder in a shell tab; palette → "New
   agent session in this folder": Agents mode opens with the dialog on
   that folder. Pick Claude Code, Start: a tab runs `claude` there. After
   the first answer go back to Agents mode: the folder shows in Projects.
2. In Files, right-click a folder (and the empty area): "New agent session
   here" → the same dialog on that folder.
3. In Agents mode press "+" in the Projects header: the dialog opens on
   the folder of the focused local tab, or `~` with none. Type `~/pro`:
   folders show under the field; pick one; type a missing path: "There is
   no such folder" and Start is off. Reopen: the last picked profile is
   proposed.
4. An SSH tab in front: the palette does not offer the command.
