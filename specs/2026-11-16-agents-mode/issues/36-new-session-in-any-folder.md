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
