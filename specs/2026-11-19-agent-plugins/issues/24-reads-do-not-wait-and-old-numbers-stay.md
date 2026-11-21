# 24 Reads do not wait for each other; old numbers stay until new ones come

Status: ready-for-agent
Blocked by: 21

## Why

Owner, 2026-11-21: "главное что бы мы видели актуальные данные и они
быстро обновлялись". On the owner's machine one read of the Codex
sessions takes about 4.4 s (`~/.codex` is 23 GB). The read runs in the
background, so nothing freezes, but at start the reads go one kind
after another (`go_on_at_start` in `crates/app/features/agents/core/src/reads.rs`):
profiles, then sessions, then limits, spend, prices, bills and account
numbers. So the slow sessions hold back limits and money on every page,
though limits do not need sessions.

## What to build

1. First look and write down, as a comment here, what the user sees
   today on a Refresh and on a later read: do the old numbers stay, or
   does a page go empty or show the spinner again?
2. The spinner "Loading" shows only while there is nothing to show yet
   (the first open). On every later read the old numbers stay on screen
   and the new ones replace them when they come, page by page, with no
   empty frame and no flicker.
3. At start, reads that do not need each other's results start
   together. Keep an order only where one read needs another (for
   example a read that needs the profiles); write each kept order and
   its reason in a doc comment.
4. A small mark that a read is on its way may show where it helps (for
   example next to Refresh), but it never hides the numbers.

## Done when

- Tests first: at start the limits read starts without waiting for the
  sessions read; on a second read the shown numbers stay until the new
  ones come; the spinner shows only when nothing was read yet.
- The checks of `AGENTS.md` pass.

## Comments
