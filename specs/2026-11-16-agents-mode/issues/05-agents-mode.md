# 05 The mode: profiles, projects, sessions, opening a tab

Status: ready-for-agent
Blocked by: 02, 03, 04

## Why

This is the first thing the user sees of Agents mode: the connected
profiles, the projects, the sessions of a project, and "Continue" and "New
session" that open a terminal tab (spec Decisions 2, 3, 6 and 7). Limits
and the profile dialog come in later issues.

## What to do

1. `Mode::Agents` in `otty-app-domain`: storage name `agents`, in
   `Mode::ALL`, with its name and icon wherever the other modes have
   theirs.
2. Two feature crates, laid out like the containers feature, each with a
   `README.md`:
   - `crates/app/features/agents/core` (`otty-app-agents`): state, intents,
     outcomes and views, no I/O. It may depend on `otty-agents` for its
     data types.
   - `crates/app/features/agents/io` (`otty-app-agents-io`): reads and
     writes `agents.json` next to `modes.json`, and runs the reads of
     `otty-agents` on a background thread.
3. The screen `screens/agents.rs` in the binary and the dispatch of the
   feature's outcomes, like `screens/containers.rs` and
   `dispatch/containers.rs`.
4. Left panel: the profiles (icon, name, state) and under them the
   projects (folder name, path as a hint, number of sessions), sorted by
   the latest session. A project whose folder is gone is marked as missing.
5. Main area: for the selected project its sessions (title, agent icon and
   profile name, time of the last change, branch), newest first, each with
   "Continue"; and "New session", which asks for the profile when there is
   more than one. With no project selected, a card that says what the mode
   is for; with no profile at all, a card that says no agent was found.
6. Status bar: the counts of projects and sessions on the left, the same
   right side as Containers mode.
7. "Continue" and "New session" switch to Terminal mode and open a command
   tab (`TabsIntent::OpenCommandTab`) with the command of `Agent::launch`,
   the project folder as the working folder and the variables of the
   command. The tab is named after the session title or the agent.
   The program must be found when OTTY was started from the desktop, where
   `PATH` has no version manager folders: run it through the user's login
   shell, the way the tabs feature starts the user's shell. A session of a
   missing project can not be opened.
8. The lists are read when the mode opens and on a "Refresh" button. While
   they load the mode shows it; a failure of one profile's sessions is
   shown at that profile and does not hide the others.
9. Agent icons: SVG files in `assets/`. When the owner has not given the
   files, draw a simple monogram for each agent and say so in the comments
   of this issue.
10. Texts go through `otty-app-i18n`.

## Done when

- Tests, written first, cover the reducer: loaded sessions become the
  projects list; selecting a project shows its sessions; "Continue" and
  "New session" report the right command, folder and variables for a
  default and an added profile; a missing project can not be opened; a
  failure of one profile keeps the other's sessions.
- Run by hand: the mode is in the All modes menu and can be pinned; a
  session of each agent continues in a new tab in the right folder.
- The checks of `AGENTS.md` pass.

## Comments
