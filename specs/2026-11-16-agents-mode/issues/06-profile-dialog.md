# 06 The profile dialog: add, create, log in, edit, hide, remove

Status: ready-for-agent
Blocked by: 05

## Why

The user must be able to connect a second account, or an agent whose config
folder is not in the default place, without making folders by hand in a
terminal (spec Decision 1).

## What to do

1. "Add profile" under the profiles in the left panel opens a dialog:
   - the agent, picked from the known agents;
   - the name;
   - the config folder, one of: a folder OTTY found (the candidates of
     issue 04), a path the user types, or "Create a new folder" with the
     proposed path, which the user can change.
   The checks of issue 04 run as the user types and are shown next to the
   field. "Add" creates the folder when asked, saves `agents.json` and
   reads the new profile's sessions.
2. A profile that is not logged in has a "Log in" button. It opens a
   terminal tab with the command of `Agent::login`. When the mode is opened
   again or refreshed, the profile's state is read again.
3. The main area shows the details of a profile selected in the left panel:
   agent, name, state, account and plan when known, config folder, and the
   buttons "Edit", "Hide" and, for an added profile, "Remove".
   - "Edit" opens the same dialog with the name, the config folder and the
     program path. It works for a default profile too.
   - "Hide" takes the profile and its sessions off the screen. A "Hidden
     profiles" row at the end of the profiles shows them again.
   - "Remove" asks first and says that the folder and its sessions stay on
     disk. OTTY never deletes a config folder.
4. A profile whose program or folder is not found says what is missing and
   offers "Edit".

## Done when

- Tests, written first, cover the reducer: adding with a found folder, with
  a typed path and with a new folder; the checks block "Add"; editing a
  default profile; hiding and showing again; removing keeps the folder; the
  "Log in" command of a default and an added profile.
- Run by hand: create a profile for Claude Code in a new folder, log in
  from the tab, refresh, see it connected.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built the profile dialog in `otty-app-agents`, tests first. Add profile
under the profiles opens a dialog over the window (an `Overlay::dialog`, like
the Containers create dialog) with the agent, the name and the config
folder: a found folder, a typed path, or a new folder whose path is proposed
from the name until the user types one; a name with no proposal (Cyrillic)
starts with an empty path. `~` stands for the home folder. The checks of
issue 04 run as the user types; a taken name, a relative path and a folder of
another profile of the same agent show under their field, an empty name or
path only keeps Add disabled. Add reports the new `agents.json` content and
the folder to create; `otty-app-agents-io` creates the folder first, then
writes the file (through a temporary file), and the profiles are read again.
The new profile is then selected. Pressing a profile shows its details in the
main area (agent, name, state, account and plan when known, config folder,
program path, what is missing) with Log in (only when not logged in), Edit,
Hide and, for an added profile, Remove. Log in opens a terminal tab in the
home folder with `Agent::login`. Edit opens the same dialog with the name,
the folder as a path and the program path, for a default profile too. Hide
and Show save the hidden flag; Hidden profiles (N) at the end of the profiles
opens a list with Show. Remove asks first and only drops the entry; nothing
deletes a folder. Esc cancels both dialogs. Texts are in all ten languages.

Choices: a default profile saves only what differs from the agent's own (a
name other than the agent's, a folder only when the user changed it, a
program path), so one that follows `CLAUDE_CONFIG_DIR` keeps following it.
When `agents.json` could not be read, no change is written over it: the
dialog or a toast says to fix or remove the file. A save that comes while
the profiles are read is followed by another read. A program path that is a
plain name (`claude-beta`) is kept as typed, for the shell to find.

Owner decisions of the same day, done here: Agents mode is pinned by default
next to Containers (`DEFAULT_PINNED`; a saved `modes.json` keeps its own
list), and Agents mode has no right panel. There was no way for a mode to
hide the right panel, so `Mode::has_right_panel` says which modes have one
and the sidebar takes the active mode: in Agents mode the panel is hidden,
its toggle and its edge do nothing, and its width is kept for the other
modes, where it shows again as it was.

Follow-up: in Agents mode the right panel's toggle in the title bar is
disabled (drawn faded, a press sends nothing). The flaky
`program_lookup::tests::each_name_is_asked_once_per_read` (and the `load.rs`
tests that run a fake shell) failed with "Text file busy": a test wrote its
fake shell while another test thread was starting a program, and the child
held the file open for writing. A temporary print of the spawn error showed
`ExecutableFileBusy` on run 25 of a loop. The scripts are now written by a
`/bin/sh` child (`testing::executable_script` in `otty-app-agents-io`), so
the test process never holds them open; 200 runs in a loop passed. One
`load.rs` test also dropped the folder of its second fake shell before using
it; the folder now stays.
