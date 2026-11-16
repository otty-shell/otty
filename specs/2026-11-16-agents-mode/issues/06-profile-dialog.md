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
