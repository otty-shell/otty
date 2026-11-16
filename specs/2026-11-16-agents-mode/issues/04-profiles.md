# 04 Profiles: the saved list, default profiles, found and created folders

Status: ready-for-agent
Blocked by: 01

## Why

The user connects profiles, not agents: one agent with one config folder
and a name (spec Decision 1). A second Claude Code account in
`~/.claude-semrush` is a second profile. This issue is the logic of the
list of profiles; the dialog is issue 06.

## What to do

All in `otty-agents`:

1. The content of `agents.json`: the profiles the user added, and what they
   changed in a default profile (name, config folder, program path,
   hidden). Functions that turn it to and from text. It holds paths and
   names only, never a secret. A file from a newer version with an unknown
   agent or field loads without that entry instead of failing.
2. The list of profiles to show: for each known agent its default profile
   (config folder from `AgentEnv`: the agent's variable, else the default
   folder under the home folder), with the user's changes applied, then
   the added profiles. A default profile whose folder does not exist and
   whose program is not found is left out.
3. Candidates for the "add" dialog: the folders directly inside the home
   folder that look like a config folder of the given agent and are not a
   profile yet. Each agent says what its config folder looks like (a new
   method of `Agent`; for Claude Code a folder with `projects/` and
   `.claude.json`, for Codex a folder with `auth.json` or `sessions/`).
4. A proposed path for a new profile from its name: `~/.claude-semrush` for
   Claude Code and "Semrush". Letters, digits and `-` only; a name that
   gives nothing usable has no proposal.
5. Creating the folder of a new profile. It fails with a clear error when
   the path exists and is not an empty folder.
6. Checks of what the user typed: the name is not empty and not used by
   another profile; the folder is not the folder of another profile of the
   same agent.

## Done when

- Tests, written first, cover each point above, the file ones on a
  temporary folder: saving and loading; an unknown agent in the file; the
  default profile with and without the variable; a hidden default profile;
  candidates found and already used ones left out; the proposed path;
  creating a folder and the refusal for a folder with files in it; the
  checks of the name and the folder.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16 (coordinator)

From issue 02: `launch` and `login` set `CLAUDE_CONFIG_DIR` / `CODEX_HOME`
only for an added profile (`!profile.is_default()`). A default profile whose
folder the user moved somewhere else would then start the agent in the
wrong folder. Fix it here, for both agents: a profile knows whether its
folder is the agent's own default folder (the one from
`Agent::default_config_dir`), and the variable is set whenever it is not.
Do not set it for a default profile in its default folder: with the
variable set, Claude Code looks for `.claude.json` inside the folder instead
of next to it, and the login of the default profile would be lost.
