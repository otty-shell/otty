# 09 Tokens spent, both agents

Status: ready-for-agent
Blocked by: 05

## Why

Limits in percent do not exist for every account: the owner's Codex account
pays per use and has no windows. The number of tokens spent can always be
counted from the session files, with no network and no login (spec Goal 2
and Decision 4). Tokens only; money is out of scope.

## Where the numbers are

- Claude Code: records of type `assistant` have `message.usage` with
  `input_tokens`, `output_tokens`, `cache_read_input_tokens` and
  `cache_creation_input_tokens`. One answer of the model can be written as
  several records: count a pair of `message.id` and `requestId` once.
  Helper agents' files under `<session id>/subagents/` count too.
- Codex: records of type `event_msg` whose payload is a `token_count` have
  `info.last_token_usage` (this answer) and `info.total_token_usage` (the
  whole session so far) with `input_tokens`, `cached_input_tokens`,
  `output_tokens`. Helper agents' session files count too.

CodexBar's `docs/claude.md` and `docs/codex.md` describe the same counting
in more detail.

## What to do

1. A `SpendCounter` for each agent that returns `Spend`: tokens in and out
   (cached ones shown apart), for today and for the last 30 days, by the
   time of each record in the user's time zone.
2. Reading every file in full on every refresh is too slow. Keep, for each
   file, what was counted together with the file's size and change time,
   and count again only the files that changed. Files not changed in the
   last 30 days are not opened at all.
3. Tolerant reading: a broken line or a record with no numbers is skipped.
4. In the app: count when the mode opens and on "Refresh", on a background
   thread, and show the numbers in the profile's details and, short, in its
   row when the profile has no limit windows.

## Done when

- Tests, written first on made-up files, cover for each agent: a session
  with several answers; the same answer written twice (Claude Code);
  records on both sides of midnight and of the 30 days; a helper agent's
  file; a compressed Codex file; broken lines; a file that did not change
  is not counted again.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16 (coordinator)

From issue 01: `TokenCount` has only input and output tokens; add the
cached ones here. `SpendCounter` takes no "now"; give it the current time as
a parameter so the tests do not depend on today's date.

### 2026-11-16 (coordinator, after the design)

The dashboard (issue 16) needs more than two totals. `Spend` gives, for
today, the last 7 days and the last 30 days, tokens in, cached and out; a
total per day for the last 30 days (for the bars); a total per project for
the last 30 days (Top projects); and a total per model for the current month
(By model, used by issue 12 for money). Counting stays one pass over the
changed files; the breakdowns are kept with each file's counts.
