# 25 OTTY remembers the session files it read and reads again only changed ones

Status: ready-for-agent
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
