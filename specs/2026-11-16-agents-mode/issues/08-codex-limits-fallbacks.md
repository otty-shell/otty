# 08 Codex limits from `app-server` and from the session files

Status: ready-for-agent
Blocked by: 03, 07

## Why

The HTTPS request of issue 07 stops working when the token is stale or the
address changes. Codex has two more ways to tell the limits (spec Decision
5), tried after it.

## What to do

1. Second source: `codex app-server`.
   - Start `codex -s read-only -a never app-server` with the profile's
     `CODEX_HOME`, and talk JSON-RPC over its input and output, one JSON
     object on a line: `initialize` (with `clientInfo`), the `initialized`
     notice, then `account/rateLimits/read`. The answer's `rateLimits` has
     `primary` and `secondary` (`usedPercent`, `windowDurationMins`,
     `resetsAt`), `credits` (`balance`, `hasCredits`, `unlimited`) and
     `planType`.
   - Send only these requests. `thread/list` is forbidden (spec Decision 5).
   - Give it a time limit. On the limit, close its input, ask it to stop,
     then kill it. Never leave the process running.
   - The process runs behind a small trait, so the source is tested with
     canned answers and no real `codex`.
2. Third source: the newest session file of the profile. Codex writes a
   `token_count` record with a `rate_limits` object (`primary`,
   `secondary`, `credits`, `plan_type`) after each answer of the model.
   Take the last such record of the newest file, plain or compressed. The
   numbers are as old as that session: `Limits` carries the time of the
   record and the profile shows it.
3. Order the three sources in `Codex::limit_sources()`: HTTPS, `app-server`,
   session file.

## Done when

- Tests, written first, cover: `app-server` answers; it answers with an
  error; it does not answer in time and is stopped; the program is not
  found; unknown notices between the answers are skipped; the session file
  source with a record, with no record, with no session files; the order
  of the three sources.
- No test and no experiment starts `codex` against the real `~/.codex`.
- The checks of `AGENTS.md` pass.

## Comments
