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

### 2026-11-16

Added `Mode::Agents` (saved as `agents`, after Containers in `Mode::ALL`,
not pinned by default) with its name, icon, palette entry and texts in all
ten languages. Built two crates like the containers feature:
`otty-app-agents` (state, reducer, view models and views, no I/O) and
`otty-app-agents-io` (reads `agents.json`, finds each profile's program and
state, reads the sessions of one profile and finds the project folders that
are gone, each on a thread of its own). The binary has `screens/agents.rs`
and `dispatch/agents.rs`. Tests were written first and seen failing; they
cover the cases of "Done when".

How it works: the app makes one `known_agents()` list at start and keeps it
in the feature. Entering the mode or Refresh reads the profiles; each shown
profile then has its sessions read on its own, so one failure stays at that
profile, and a failed read keeps the sessions read before. Continue and New
session build the command with `Agent::launch` and open a command tab
through the user's shell started as `-l -i -c "exec '<program>' '<arg>'…"`,
in the project folder, with the profile's variables. This comes from a new
`ShellSession::command_session` in `otty-app-shell`. Whether a program is
found is asked of the same shell (`command -v`), so the profile list and
the tab agree.

Choices and deviations: the shell runs as both a login and an interactive
shell, because version managers such as `nvm` add their folder in
`~/.zshrc` or `~/.bashrc`, which a login shell alone does not read. It runs
without the OTTY shell integration, because the integration's `ZDOTDIR`
and `--rcfile` would replace the user's own start files. A profile without
a program path whose program is not found by the shell is "program not
found"; it can not start or continue a session. The agent tab stays open
when the agent fails (`CloseOnSuccess`), so its error can be read. A new
session's tab is named after the profile, which is the agent's name for a
default profile. With more than one profile, New session shows the
profiles as buttons in the main area, not in a dialog. Session times show
as "5 min ago". The left side of the status bar says "agent sessions", so
it is not confused with the terminal sessions button on the right. This
issue only reads `agents.json`; writing it comes with the profile dialog
(06). The right panel shows nothing. The text "No other modes yet" of the
All modes menu no longer says agent work will appear there.

Icons: no agent icons were given, so OTTY draws simple monograms in
`assets/svg/`: `agent-claude-code.svg` ("CC" in a rounded square) and
`agent-codex.svg` ("Cx"), and a robot head for the mode,
`mode-agents.svg`. Replace them with the real icons when the owner has
them.
