# 25 OTTY remembers the session files it read and reads again only changed ones

Status: done
Blocked by: 24

## Why

Owner, 2026-11-21: up-to-date numbers that refresh fast. Today the
session list of a profile is read whole every time: the Codex part goes
over every session file of `~/.codex` (23 GB on the owner's machine,
about 4.4 s per read) though only a few files changed since the last
read. The tokens already work the faster way: the part names its spend
places, OTTY walks the files, remembers what it read and asks the part
again only for files that changed (Decision 6). Sessions should work
the same way, so a plugin does not need its own memory between reads
(it has none).

## What to build

1. Measure first, on the owner's real files, how long a session read
   takes today and where the time goes; write it as a comment here.
2. The agent contract names where the sessions are (a glob, like the
   spend places) and reads one file or database into the sessions it
   holds. OTTY walks the files, keeps what each unchanged file gave
   (by path, size and change time) and asks the part only for new or
   changed files; a deleted file drops its sessions. Remove the call
   that reads the whole list. If a database holds the sessions (as
   OpenCode's), find the simplest way to read only what changed, or
   say why it is not needed (measure).
3. Keep the remembered results in memory for now. If the measure shows
   that the first read after OTTY starts is still slow, ask the owner
   before keeping them on disk.
4. Update the template, the official plugins, the SDK test host, the
   docs (`Write a Plugin.mdx`) and `spec.md`.

## Done when

- Tests first: a second read with no changed file asks the part for
  nothing; one changed file is read again alone; a new file adds its
  sessions; a deleted file removes them; the list is the same as a
  full read.
- The second read of the owner's Codex sessions is measured and much
  faster than 4.4 s (write the number here).
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-21, measured before the change

Measured on the owner's real `~/.codex`, read only, through OTTY's own
code path: the ignored test
`measure_the_sessions_read_of_the_real_folders` in
`crates/app/features/agents/io/src/load.rs` loads the embedded OpenAI
plugin and reads the sessions as the app does (release build, `nice`).

- 2,850 session files (10.8 GB; 2,038 of them `.jsonl.zst`), 1,437
  sessions listed.
- First read 6.0 s, then 4.5 s on every read.
- Where the time goes: almost all of it in the host decoding the first
  line of each compressed file. A host-side measure of `read-lines` alone
  over the same files took 4.3 s, 4.3 s of it on the `.zst` files. The
  first lines are large (58 MB in all, about 20 KB each), and the host's
  pure-Rust zstd decoder (`ruzstd`) is about ten times slower than the C
  one (the same first lines decode in 0.4 s with C zstd). Listing the
  files takes about 30 ms, the plain first lines about 30 ms, the index
  (230 KB) under 1 ms.

### 2026-11-21, done

- **Contract** (`agent.wit`, changed in place): `sessions` is gone. A part
  names `session-places(profile) -> list<place>` and reads one file with
  `read-sessions(profile, item: place-item) -> result<list<session>,
  read-error>`. The spend records are renamed to fit both:
  `spend-place` → `place { glob, kind: place-kind }`, `spend-kind` →
  `place-kind`, `spend-item` → `place-item { file: file-info, kind }`;
  the item now carries the size and the change time OTTY listed, so a
  part needs no listing of its own (Claude Code reads the ends of a file
  by its size; Codex takes the file time). `session.folder` and
  `session.title` are optional: OTTY joins what all files tell about one
  id (the latest time, and the folder and the title of the latest record
  naming them), leaves out a session no file gives a folder, and titles
  one with no title by its id.
- **OTTY** (`crates/agents/src/session_files.rs`, `plugin_sources.rs`,
  `plugin_places.rs`): walks the session places with a `FileMemory`, as
  for the tokens, in memory only; a new or changed file is read alone, a
  deleted one drops its sessions. The listing of places is shared with
  the spend counter. `has_sessions` reads through the same memory.
- **Codex**: the session files and `session_index.jsonl` are the session
  places; a session file tells id and folder, the index tells names and
  times. **Claude Code**: each `projects/*/*.jsonl` file tells its one
  session. **Template**: each `sessions/*.jsonl`.
- **OpenCode**: the database and its write-ahead log are the session
  places (as for the answers); a change of either reads the whole
  session table again. Measured on the owner's real database (27 MB, no
  sessions in it today): first read 12 ms, second 0.2 ms. One query over
  a table of a few thousand rows stays in milliseconds, so reading only
  changed rows is not needed. One known gap: a session deleted in
  OpenCode stays listed until the database file itself changes (the log
  is copied into it), since the database file's remembered sessions still
  name it; the tokens have the same gap today.
- **SDK test host**: `TestHost::place_items(part, &places)` lists the
  files of places as OTTY does, for plugin tests.
- Docs: "Write a Plugin" and `spec.md` (Decision 6, SDK sketch, WIT
  sketch, host work).

Measured after the change, same test and files:

| Read | Before | After |
| --- | --- | --- |
| First read after start | 6.0 s | 4.5–5.1 s |
| Second read, nothing changed | 4.5 s | 26 ms |
| Third read | 4.5 s | 25 ms |

The list is the same: 1,437 sessions before and after. **The first read
after start is still slow** (about 4.5 s): it is the zstd decoding of the
first lines. Keeping the remembered results on disk, or a faster zstd
decoder in the host (a C binding is a new dependency), are the owner's
call; neither is done here.
