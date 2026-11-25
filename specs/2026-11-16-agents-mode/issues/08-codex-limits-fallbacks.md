# 08 Codex limits from `app-server` and from the session files

Status: ready-for-agent
Blocked by: 03, 07, 11

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

### 2026-11-16 (coordinator, after the design)

Blocked by 11 too. Each source returns `Limits` in the billing of spec
Decision 9: windows when the answer has them, credits when it has a
balance (`hasCredits`, `balance`, `unlimited`; no total, so no bar). A
source that answers is named in "Where the numbers come from" on the
dashboard (issue 16).

### 2026-11-16

Built tests first and saw them fail. `Codex::limit_sources()` now gives
"usage request", "app-server" and "session file", in that order. The shared
rule of issue 11 (windows, else credits, else "unexpected answer") moved to
`codex/rate_limits.rs`, so all three sources follow it.

`app-server` (`codex/app_server.rs`) starts `codex -s read-only -a never
app-server` (the profile's program path when it has one, `CODEX_HOME` as
for the agent tab) and sends only `initialize`, `initialized` and
`account/rateLimits/read`. Codex leaves out the `"jsonrpc": "2.0"` field, so
OTTY does too. The answer is matched by its id; notices, Codex's own
requests and lines that are not JSON are skipped. The whole talk has a time
limit of 10 seconds, because the user's shell reads its start files first.
Not started, ended, refused (with Codex's message) or too slow is "not
available"; the program is stopped in every case.

The process sits behind two small public traits in `otty-agents`
(`StartProgram`, `RunningProgram`), so the source is tested with canned
lines. `Agent::limit_sources` now takes the starter. The real one lives in
`otty-app-agents-io` (`agent_program.rs`): it runs the program through the
user's login shell, the way the agent tab does (`exec_script` of
`otty-app-shell` is now public for it), and reads the output on a thread of
its own. Stopping closes the input, waits a second, sends `SIGTERM` with the
system `kill` program, waits a second, then kills; the program is also
stopped when it is dropped. Its tests use made-up scripts and a made-up
shell, never `codex`.

"session file" (`codex/session_limits.rs`) reads the session file that
changed last, plain or compressed, and takes its last `token_count` record
with a `rate_limits` object; a record whose limits are `null` is skipped.
Older Codex versions wrote `resets_in_seconds`, which is counted from the
record. `Limits` has a new `recorded_at()`: the time of the record, or of
the file when the record has none. The profile details show it as
"Recorded in a session 30 min ago" (a new text in every language).

Deviation: `SIGTERM` is sent by running `kill`, because sending a signal
from Rust needs `libc` or `nix`, which `otty-app-agents-io` does not depend
on.

After the coordinator's follow-up, "session file" walks the 10 newest
session files, newest first, and takes the newest record found; a file that
can not be read is skipped, and only when none of the 10 has a record is it
"not available".
