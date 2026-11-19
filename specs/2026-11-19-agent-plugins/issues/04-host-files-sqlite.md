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
