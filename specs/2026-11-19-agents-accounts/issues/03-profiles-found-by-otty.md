# 03 Profiles found by OTTY and the first-start report

Status: ready-for-agent
Blocked by: 01

## Why

The user should not add a profile OTTY can find (spec Decision 2).

## What to build

1. Find extra folders of known agents in the home folder (`~/.claude*`,
   `~/.codex*`) that hold the agent's login or sessions, and list them as
   profiles named after the folder ("claude-work" → "Work"). The user's
   saved profiles stay; a found folder that is already a saved profile is
   not listed twice.
2. The **Found by OTTY** dialog on the first start (and never again unless
   the user opens it from All accounts ⋯ later): agents table (agent,
   version, folder, accounts count), accounts table (account, found in,
   reads, now), and the needs split into "Accounts need you" and "Profiles
   need you".
3. The agent's version is read from its files where it writes it (Claude
   Code records, Codex session meta) or from `--version` of the program
   already found; shown in the dialog.

## Done when

- Tests first: folders found and named; a folder without a login or
  sessions skipped; no duplicate of a saved profile; the dialog view model.
- The checks of `AGENTS.md` pass.

## Files

`crates/agents/src/profile_list.rs`, `config_dir.rs`; feature core (new
dialog) and io; i18n.

## Owner checks by hand

`~/.claude-semrush` shows up as a profile with no setup; the first start
shows Found by OTTY.

## Comments

Built (2026-11-19):

- Found profiles (`otty-agents`, `profile_list.rs`): after the default and
  the added profiles, every folder in the home folder named after the
  agent's folder with a `-` or `_` and more (`~/.claude-work`,
  `~/.codex_team`) that holds a login (`detect` says connected) or a
  session file (`Agent::has_sessions`, no file is read). It is named after
  the rest of the folder name with capital words ("Work", "My Team"); when
  another profile has that name, "Claude Code · Work". A folder that is
  already the folder of a default (also moved) or added profile of the same
  agent is skipped. `Profile::found` and `is_found`: such a profile is not
  saved; Edit saves it as an added profile, so it is never listed twice,
  and Remove is off for it. Only folders are read; nothing is written.
- Version: `Agent::version` reads the newest session: Claude Code the last
  record with `version` in the newest `projects/*/*.jsonl`, Codex the
  `cli_version` of the `session_meta` of the newest session file (up to 10
  are tried). The read of the profiles never starts a program. For a
  profile with no session version whose program was found, the feature
  asks `LoadVersion(agent)` (io `load_version`, `<program> --version` in
  the login shell) only while Agents mode is on screen, once per agent in
  a run; a refresh does not ask again. `LoadedProfile::version`.
- `agents.json` gets `"found_report_shown": true`, written when the report
  is closed.
- Feature: the Found by OTTY dialog (`found_report.rs`, its model and
  view). It opens after a read of the profiles that found any, when
  `agents.json` was read and does not say it was shown, at most once per
  run, and shows when Agents mode is on screen. Tables: Agent, Profile,
  Version, Folder, Accounts; Account, Found in, Reads, Now. "Accounts need
  you": an organization with no admin key, button Add admin key (opens Edit
  profile, which has the field). "Profiles need you": needs login, program
  not found, folder not found, with Log in when it can. Done or Esc closes
  it; the button of a need closes it first, then acts.
- Binary: the overlay `AgentsFoundReport` in the guards (Esc = Done).
- i18n: 22 keys in the 10 languages. Docs: the Profiles part of the
  Overview.

Decided differently:

- The agents table has a Profile column next to Agent (the prototype puts
  the profile name under Agent): "Work" alone does not say which agent.
- Now shows the short number of the account's first profile, as its left
  panel row ("42 %", "$1.20 today"), until issues 04 and 05 give accounts
  their own numbers.
- Opening the report again from All accounts ⋯ is left to issue 04, which
  makes that page.
- Only the needs that exist today are listed; login expired, keyring and
  format changed come with their issues.
- The `--version` answer is kept in the feature per agent (its program as
  OTTY starts it, by name through the login shell), not per resolved path.
