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
