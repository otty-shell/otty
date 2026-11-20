# 04 Host operations: files, lines, SQLite, environment, clock, log

Status: ready-for-agent
Blocked by: 03

## Why

Spec Decision 6 and the table "Host operations". Reading files and
databases is most of an agent part's work and decides its speed.

## What to build

In `otty-plugin-host`, each operation checked on every call against the
`Permissions` of the part being called (issue 02), never another part's:

1. `files.list(glob)`: paths, sizes, change times; at most 100,000.
2. `files.read(path, max)`: at most 4 MiB; refused for a file listed in
   `keys` (it must be read with keys replaced, issue 05).
3. `files.read-lines(path, offset, max, markers)`: complete lines from a
   byte offset, about `max` bytes per batch, only lines holding one of
   the markers found with `memchr::memmem` (add `memchr` as a direct
   dependency; it is in the lock), a count of skipped lines, the next
   offset, at-end. The last line without a newline is handed out only at
   the end of the file. Files starting with the zstd magic bytes are
   decoded on the fly (`ruzstd`, moved from `otty-agents`); offsets then
   count decoded bytes.
4. `sqlite.query(db, sql, params)`: open with `SQLITE_OPEN_READ_ONLY` and
   `PRAGMA query_only`, refuse a statement that is not read-only
   (`Statement::readonly`), stop after 10 s with an `InterruptHandle`, at
   most 50,000 rows; values as the WIT variant. With the `rusqlite`
   feature `hooks` (approved 2026-11-20), an authorizer refuses `ATTACH`
   and reading a key column the part declares as plain text; issue 05
   gives those columns back as key handles.
5. `env.var(name)` for listed variables, `env.home()`; `clock.now-ms`;
   `log.write` into the capped log.
6. Move the generic code these replace out of `otty-agents` only where it
   is not used by in-core readers any more; otherwise copy and leave a
   note for issue 17.

## Done when

- Tests first, on made-up files in a temporary folder: globs and denials
  (outside the globs, `..`, symlink leaving the glob); batches end on
  complete lines; markers keep and skip the right lines, including a
  marker split across a batch edge; a zstd file gives the same lines as
  its plain copy; a write statement, `ATTACH` and a declared key column
  read as text are refused; a slow query stops; an unlisted variable
  reads as none; a glob or variable of another part of the same plugin
  is refused.
- A bench note in the issue comments: `read-lines` with markers on the
  spike's 100 MB synthetic fixture is at least as fast as the spike's
  (416 MB/s through a plugin).
- The checks of `AGENTS.md` pass.

## Files

`crates/plugins/host/src/` (files, lines, sqlite, env, clock, log),
`crates/plugins/host/Cargo.toml`.

## Comments

### 2026-11-20: implementation notes

- `files.list` walks folder by folder along the glob and lists the files
  the part may read, read with keys, or query, as the SDK test host does.
  A glob that is not absolute or holds `..` is refused; files outside the
  part's globs are left out, not refused. Symlinks to folders are not
  followed.
- Symlinks: a path is reachable only when the real file it leads to also
  matches the part's globs, checked as if written from the home folder or
  the variable folder it lies in (a symlinked `/home` keeps working; an
  agent folder the user symlinked into a dotfiles folder does not).
- `files.read-lines`: at most 1 MiB asked per batch; a line longer than
  16 MiB is left out and counted as skipped. The decoder of the last
  zstd file stays open for the next batch of the same call batch. Only
  single-frame zstd files are decoded, as in `otty-agents`.
- Note for issue 17: nothing was moved out of `otty-agents`. Its in-core
  readers still use `ruzstd` (`codex/session_file.rs`) and `rusqlite`
  (`open_code/database.rs`); the host has its own code for both.
- Open: `memchr` was not added (not in the coordinator's list of approved
  dependencies; the permission check refused it). The marker search looks
  for the first byte and then compares the rest, as the spike's faster
  version did; swapping in `memchr::memmem::Finder` is local to `holds`
  in `operations/lines.rs`.
- Open: the bench note. Building a bench outside the worktree was refused
  in this run, so `read-lines` on the spike's 100 MB fixture is not
  measured yet.
- 2026-11-20: the marker search now uses `memchr::memmem::Finder`, one per
  marker built once per call (owner approved `memchr`); the SDK test host
  searches the same way. Key columns in `sqlite.query` now come back as
  handles; the rule is in issue 05's comments.
