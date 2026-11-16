# 03 Codex: detect, sessions from plain and compressed files, launch command

Status: ready-for-agent
Blocked by: 01

## Why

Codex is the second agent of the first version. Its sessions are read from
its files only; `codex app-server` is never asked for the list (spec
Decision 5 explains what that request did to the owner's session index).

## What Codex keeps on disk

Checked with the files written by Codex 0.130 and 0.163 on Linux:

- The config folder is `CODEX_HOME`, else `~/.codex`.
- `<config>/auth.json` holds the login: `auth_mode`, `OPENAI_API_KEY`
  (may be null), `tokens` (`access_token`, `account_id`, ...),
  `last_refresh`.
- `<config>/sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl` is one session.
  Older ones are compressed to `.jsonl.zst` with the same name.
  `<config>/archived_sessions/` holds archived ones in the same format.
- The first line is a record of type `session_meta`; its `payload` has `id`
  (the same id as in the file name, the one `codex resume` takes), `cwd`,
  `source` and `git`. `source` is a string (`cli`, `vscode`, `exec`, ...)
  for a session the user started and an object for a helper agent's
  session.
- `<config>/session_index.jsonl` has one line for each named session: `id`,
  `thread_name`, `updated_at`.

## What to do

1. Add the `ruzstd` dependency to `otty-agents` (approved by the owner, spec
   Decision 8). `cargo deny check` must pass.
2. `Codex` implements `Agent`: descriptor, default config folder, `detect`,
   `launch` (`codex`, or `codex resume <id>`; an added profile sets
   `CODEX_HOME`, the default one sets nothing), `login` (`codex login` with
   the same variable). No limit sources and no spend counter yet.
3. One session source that reads the files:
   - both folders, plain and compressed files;
   - only the first line of a file is needed: unpack a compressed file as a
     stream and stop after the first line;
   - sessions whose `source` is an object are not listed;
   - title from `session_index.jsonl`, else the id; time of the last change
     from the index, else the file time; branch from `git` when it is
     there;
   - tolerant reading as in issue 02: a file whose first line can not be
     read is skipped and the others are still listed.
4. Reading thousands of files on every refresh is slow. Keep, in memory for
   now, what was read from a file together with its size and change time,
   and read again only the files that changed. The source owns this.
5. Add `Codex` to `known_agents()`.

## Done when

- Tests, written first on a temporary config folder with made-up files
  (a compressed one is made by the test data, not copied), cover: a plain
  session; a compressed session; a helper agent's session is not listed; a
  name from the index; a file with a broken first line; a missing
  `sessions` folder gives an empty list; a file that did not change is not
  read twice; `detect` and the launch commands as in issue 02.
- No test and no experiment starts `codex` against the real `~/.codex`.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Added `Codex` in `crates/agents/src/codex/`: the agent itself (`agent.rs`),
the login check (`detect.rs`), the walk over the two session folders
(`sessions.rs`), the reading of the first line of one file, plain or
compressed (`session_file.rs`), the names in `session_index.jsonl`
(`session_index.rs`) and the memory of what was read (`cache.rs`). It is the
second entry of `known_agents()`. The crate now depends on `ruzstd` 0.9.1
with its default features; its own encoder makes the compressed files in
the tests, so no fixture is committed.

Pieces with the same meaning for both agents are now shared: the command
that starts an agent with its config folder (`profile_command` in
`command.rs`, so issue 04 changes the rule in one place), the cut of a
title (`title.rs`) and the reading of a small JSON file (`json_file.rs`).

Choices the issue left open: `detect` says connected when `auth.json` has a
non-empty `tokens.access_token` or a non-null `OPENAI_API_KEY`; the key and
the token are only checked and never kept; account and plan stay empty.
Only files named `rollout-*.jsonl` or `rollout-*.jsonl.zst` are read, at any
depth of both folders, without following links. A first line longer than
1 MiB, or one that is not a `session_meta` record with an id and a `cwd`,
skips the file; a session with no `source` counts as the user's. In the
index the last line of an id wins, and the name is cut like a Claude Code
title (blank name: the id). What was read is kept for each config folder by
file path, size and change time; files that are gone are forgotten on the
next refresh. This memory lives in the `Codex` value and is shared with the
sources it gives, so the app must keep one `Codex` (one `known_agents()`
list) for the whole run. After review with the owner, a session id found
more than once (in both folders, or plain and compressed) is listed once,
the entry with the newest time, and that time is the newer of the index
time and the file time.
