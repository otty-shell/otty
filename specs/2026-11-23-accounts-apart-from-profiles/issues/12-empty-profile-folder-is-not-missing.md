# 12 An empty profile folder is not "folder not found"

Status: done
Blocked by: 11

## Why

Owner, 2026-11-23: a profile the owner added showed something like
"folder not found", though it works and the folder is there, only still
empty. For a new profile the agent should say what it needs, such as a
login, and that depends on the agent: OpenCode needs no login before it
starts working.

Cause: the plugins' `detect` decides "config folder not found" by listing
files (`folder_exists` in `plugins/openai/src/detect.rs`,
`plugins/anthropic/src/detect.rs`, `plugins/opencode/src/opencode.rs`,
`plugins/template/src/agent.rs`), and the host's file listing never lists
a folder itself, so an empty folder counts as missing.

## What to build

1. **A folder that exists is never "folder not found".** Give the parts a
   way to tell an existing empty folder from a missing one. Prefer the
   simplest sound way in this codebase: either OTTY checks the profile
   folder on disk itself before it calls `detect` (it knows the path of
   every default and added profile, the check is not agent-specific) and
   reports "folder not found" only when the folder is truly missing, or
   the host's files interface gains a check that a folder exists (inside
   the part's permissions). Pick one, say why in the comment, and make
   "folder not found" mean only that.
2. **The plugin decides what an empty folder needs.**
   - Codex (OpenAI plugin) and Claude Code (Anthropic plugin): an empty or
     login-less folder is `not-logged-in` with their login need ("No
     login", `codex login` with `CODEX_HOME`, `claude auth login
     --claudeai` with `CLAUDE_CONFIG_DIR`, in that folder).
   - OpenCode: an empty data folder is found, with no need, since OpenCode
     works with no login.
   - The template shows the pattern.
   Bump and rebuild the changed plugins.
3. The profile page of a found profile with no sessions yet says so plainly
   (as "installed, not used yet" does today), never "folder not found".

## Done when

- Tests first: an added empty folder of Codex and of Claude Code is "No
  login" with the login command for that folder; an added empty OpenCode
  folder is found with no need; a folder that does not exist is "folder
  not found"; the profile page of an empty found profile shows no error.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-24, done. "Folder not found" now means only that the folder is
  not there.

  The way chosen: OTTY checks the profile folder on disk itself, before
  it asks the agent part. `otty_agents::profile_status` first looks for
  the program, as before, then says `ConfigDirNotFound` when the folder
  of the profile is not a folder, and only then calls `detect`. Why this
  way and not a new host call: OTTY knows the folder of every default and
  added profile, the check is the same for every agent, it is one line in
  one place, and no part can get it wrong again. A host call that tells
  whether a folder exists would have needed a new function in
  `files.wit`, a rule for which folders a part may ask about, and every
  plugin to call it.

  Contract (changed in place): `profile-status` of `agent.wit` lost
  `config-folder-not-found`, so a part can no longer say it. A part is
  asked only about a folder that is there, which may be empty, and it
  decides what such a folder needs. `PluginAgent::detect` lost the
  mapping, and the test agent Coder its own folder check.

  Plugins: Codex has no folder guess any more, so an empty folder holds
  no login and is "Needs login" with `codex login` and `CODEX_HOME` set
  to the folder (added profiles; the default sets no variable). Claude
  Code likewise, and its login need now runs `claude auth login
  --claudeai` (with `CLAUDE_CONFIG_DIR` for an added folder) in place of
  a bare `claude`, the command the owner chose for Log in with Claude
  Code. OpenCode is always found, so an empty data folder is found with
  no need. The template shows the pattern: no `auth.json` is "not logged
  in" with its login need, and its doc says an agent with no login would
  say found. `openai` 0.2.12 -> 0.2.13, `anthropic` 0.2.12 -> 0.2.13,
  `opencode` 0.1.8 -> 0.1.9, template `example` 0.1.6 -> 0.1.7, rebuilt
  into `assets/plugins/`. `deepseek` and `openrouter` export no agent
  part, so they did not change. The four test plugins and `template.wasm`
  of the host were rebuilt for the changed WIT.

  Screen: the profile page of a found profile with no sessions already
  said "<agent> has no sessions yet" with no need and no error, so it
  only got a test.

  Docs: Overview (a profile shows "folder not found" only when its folder
  is not there), Write a Plugin (`detect`), and the OpenAI, Anthropic and
  OpenCode pages (the login need and its command, an empty data folder
  is found).

  Tests, written first and failing: `profile_status` says
  `ConfigDirNotFound` for a missing folder and for a file in its place
  without asking the agent, and asks the agent about an empty folder;
  the plugin tests of an empty Codex, Claude Code, OpenCode and template
  folder; and one test over the built plugins in `otty-app-agents-io`
  (`load.rs`): added empty folders of Codex and Claude Code are "Needs
  login" with the login command and the folder's variable, an added empty
  OpenCode folder is found, and an added folder that is not there is
  "folder not found". The profile page test of an empty found profile
  passed at once.
