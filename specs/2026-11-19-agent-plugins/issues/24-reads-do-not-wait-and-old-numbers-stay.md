# 24 Reads do not wait for each other; old numbers stay until new ones come

Status: done
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

### 2026-11-21, what the user sees today (step 1)

Read from the code and the tests of `crates/app/features/agents/core`,
not from the running app.

- **First open.** The main area shows the spinner "Looking for agents…"
  and the left panel and the status bar say "Loading" only until the
  first read of the profiles answers (`Profiles::is_loaded`: the read
  has a time). After that a page shows at once, and its numbers fill in
  as each read answers; before a read answers its place is empty, as
  before any read.
- **Refresh.** The profiles are read again, then every other kind at
  once. Nothing goes empty and the spinner does not come back: a profile
  read again keeps its sessions, limits, tokens and bill
  (`ProfileState::take_reads_of`, tests
  `the_limits_are_kept_by_their_profile_and_survive_a_refresh`,
  `the_tokens_are_kept_by_their_profile_and_counted_again_on_refresh`),
  the prices stay until new ones come, and each new answer replaces the
  old numbers of its page when it comes. A read that fails keeps the
  last good numbers with a line that says why. Nothing on screen shows
  that a read is on its way.
- **A later read** (the 5-minute timer, or opening the mode when
  something is older than 5 minutes): the same, the old numbers stay
  until the new ones come.
- **What does change on a read, on purpose:** a profile whose config
  folder is gone loses what was read of it; a bill read with an admin
  key since changed is forgotten; when the latest bill read failed, the
  estimate shows in place of the bill (spec Decision 5); an account no
  longer listed loses its own numbers.
- **The real wait is at start.** With the mode away, the reads go one
  kind after another: profiles, then sessions, then limits, tokens,
  prices, bills and account numbers. On the owner's machine the Codex
  sessions take about 4.4 s, so the limits and the money of every page
  come at least that late, though they do not need the sessions.

So steps 2 and 4 hold today; this issue adds tests that keep them, and
changes the start order (step 3).

### 2026-11-21, done

- **Start order** (`crates/app/features/agents/core/src/reads.rs`). The
  profiles come first: every read needs them. Their answer asks for the
  sessions, the limits, the tokens and the numbers of the accounts that
  tell their own together, so the slow Codex sessions hold up nothing.
  The list prices and the bills alone wait, for the limits
  (`StartWait::Limits`): they are read for the profiles that pay with an
  API key, and a profile whose files do not tell how it pays learns it
  from its limits answer. With no limits to read they come at once. The
  old chain of kinds (`ReadKind`, `go_on_at_start`) is gone; opening the
  mode during the start still ends it and reads what is missing.
- **Old numbers stay, spinner only at first.** The code already did this
  (see the comment above); new tests keep it:
  `a_refresh_keeps_the_numbers_on_screen_until_new_ones_come` (the page
  of an account shows the same numbers through a Refresh until the new
  limits come) and `a_refresh_keeps_the_sections_and_says_nothing_loads`
  (no "Loading" in the left panel on a Refresh).
- **No mark of a read on its way** (step 4 is a "may"): left out to keep
  the change small. It can come with its own issue if the owner wants
  one next to Refresh.
- Docs: the start row of "When OTTY reads" in
  `docs/public/Agents/Overview.mdx`, with a line that the numbers stay
  until new ones come.
