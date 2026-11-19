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
