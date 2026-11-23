# 12 An empty profile folder is not "folder not found"

Status: ready
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
