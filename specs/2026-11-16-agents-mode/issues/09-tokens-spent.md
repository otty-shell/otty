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

### 2026-11-16

Built in `otty-agents`, tests first on made-up files. Both agents now give a
`SpendCounter` named "session files"; it takes a `LocalNow` (the current time
and the user's offset from UTC), so no test depends on today's date.
`TokenCount` has input (sent, not read from the cache), cached (read from the
cache) and output, plus `cache_written`, the part of the input Claude wrote to
its cache, which issue 12 needs because it is priced apart; Codex leaves it 0.
`Spend` keeps one entry per day, each split by model, from 30 days back (or
from the first of the month on its 31st day, the same rule for files) to
today, and the projects of the last 30 days. Today, 7 days, 30 days, the 30
bars and the models of the current month are worked out from those days, so
there is one source of truth.

One pass over the files: what each file told (its project and every answer
with its time, model, tokens and identity) is kept per config folder with
the file's size and change time, in the agent value, as Codex's session
cache does. A file with the same stamp is not read again; a file not changed
since the first counted day is not opened. `ClaudeCode` is no longer a unit
struct: it holds that memory, so it is `ClaudeCode::default()` now.

De-duplication, after CodexBar's docs: a Claude Code answer is the pair of
`message.id` and `requestId` (else `sessionId` and `message.id`); of the
records of one answer, in any file, the one with the most output tokens
counts. All `*.jsonl` files in the project folders count, at any depth, so
helper agents' files under `subagents/` count under their session's project.
Codex counts `last_token_usage` (else the growth of `total_token_usage`),
skips a `token_count` whose total did not change, and counts the same record
(same time and numbers) once across files, which covers the plain and
compressed copy, the archived copy and copied fork history. The model comes
from the last `turn_context`; Codex's input includes its cached part, which
is taken out.

In the app, the tokens of every shown profile whose folder exists are
counted when the mode opens and on Refresh (not on the five-minute limits
timer), on a thread of their own (`otty-app-agents-io/src/spend.rs`), kept
in the state with the last good count (Decision 5), and shown in the
profile details as "Tokens today" and "Tokens, 30 days" (in, cached, out).

Deviations: the user's offset from UTC is read from `date +%z` once per
count (no crate in the workspace knows the time zones; without an answer the
days are the days of UTC). One offset is used for the whole count, so on a
day of a summer-time change a record near midnight may land on the next or
previous day. The short number in the profile row is left to issues 14 and
16: the left panel's number follows the billing (spec Decision 6). CodexBar's
finer fork rules (`subagent_history_start_ordinal`, compact fork baselines)
are not followed; copied history is caught only when the records are exact
copies.
