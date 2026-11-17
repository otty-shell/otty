# 29 The probe trusts its own folder

Status: ready-for-agent

## Why

The owner's main Claude Code profile shows "limits not available, log in"
although the owner is logged in: the Claude Code login token in
`.credentials.json` has expired (issue 07 counts an expired token as "not
logged in" and never renews it), and Claude Code renews it only when it
runs. The hidden `/usage` probe of issue 10 runs Claude Code, but it stops
at the question "Do you trust the files in this folder?" in its own folder
`~/.cache/otty/agent-probe`, so nothing answers. The owner wants OTTY to get
the limits without opening a terminal (spec Decision 12).

## What to do

1. The probe answers the trust question with "Yes" when, and only when, the
   folder the question names is the probe's own folder (compare the path on
   the screen with the probe folder; if the screen does not name it, accept
   only when the probe was started in that folder and the question is the
   trust question). Follow how CodexBar's `ClaudeCLISession` does it (MIT,
   https://github.com/steipete/CodexBar; read with `gh api` or WebFetch,
   copies only in the scratchpad or `target/`). Any other question still
   stops the probe with "not available".
2. After accepting, the probe goes on as usual: waits for the prompt, types
   `/usage`, reads the panel. Claude Code renews its own token while it
   runs; OTTY still never reads or writes the token itself.
3. An expired token in the usage request is no longer "not logged in" but
   "login expired" with the reason "Claude Code renews it when it runs", so
   the dashboard does not tell the user to log in while the probe can still
   answer. The probe is the one to say "not logged in" when Claude Code shows
   its login screen.
4. The issue 10 Comments and the module docs of the probe say what the probe
   now writes (the trust of its folder in the profile's `.claude.json`) and
   that the owner accepted it (spec Decision 12).

## Done when

- Tests, written first, with the fake terminal: the trust question for the
  probe folder is answered and the panel is read; a trust question for
  another folder stops the probe; another question stops it; the expired
  token gives "login expired".
- No test runs `claude` or touches a real `.claude.json`.
- Run by hand: on a profile whose token has expired, open Agents mode; the
  limits show from "Claude Code /usage" within about 30 seconds, and a
  refresh a few minutes later answers from "Anthropic usage API" again.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

### 2026-11-17

Built in `otty-agents` and `otty-app-agents`; no new dependency.

How the question is recognised: the screen reading (`usage_screen.rs`)
now names the trust question as its own result, `TrustQuestion`, from the
texts it already knew ("Do you trust the files in this folder", "Yes, I
trust this folder", "Quick safety check", "Ready to code here?"). A login
or another question on the same screen still stops the reading first, so
the trust question is answered only when it is the only thing asked.

How it is answered (`trust_question.rs`, after CodexBar's
`ClaudeCLISession.workspaceTrustKeys`): the lines of the screen are read
without the spaces and box borders around them. A line that starts with
`/` or `~/` is a folder. When the screen names a folder, one of them must
be the probe's folder, written in full, from home (`~/.cache/otty/...`),
or as the system resolves its links (the probe passes both
`working_dir()` and its `canonicalize`); otherwise the probe stops with
"not available: claude asks whether to trust another folder: <folder>"
and types nothing. When no folder is named, the question is about the
folder the probe started in, which the starter always makes its own. Then
the "Yes" option ("Yes, I trust this folder" or "Yes, proceed") is found,
and the `❯` marker in the same list of options (up to a blank line): on
"Yes" Enter is pressed, above or below it an arrow moves it toward "Yes",
with no marker nothing is pressed. One key is typed for each new screen
of the question, never again on a screen that did not change. After the
question the probe waits as at the start (two seconds of a still screen)
and types `/usage` as before. A question that stays until the time limit
is "not available: claude still asks whether to trust its folder".

What CodexBar does: it trusts whenever it started in its own dedicated
probe folder (checking that the folder is not a temporary fallback and
not redirected by a link) and never compares the folder on the screen.
For the old question with no "Yes, I trust this folder" line it types
`y` and Enter, for "Ready to code here?" Enter, and Escape when it may not
trust. OTTY differs: it also compares the folder on the screen, types
only arrows and Enter by the marker, never Escape (it stops the program
instead), and leaves "Ready to code here?" unanswered unless the screen
has one of the two known "Yes" options, so such a screen ends with "still
asks". CodexBar also allows at most four keys; OTTY has no key count, the
30 second limit bounds it, and no key is typed twice for one screen.

Login expired: `SourceError` has a new case `LoginExpired(reason)`
("login expired: <reason>"). The Claude usage request gives it for an
expired token, with "Claude Code renews it when it runs". A refused
request (401/403) and the probe's login screen stay "not logged in". In
the app the source row is "login expired" (warning tone) with the reason
as its tooltip, and the "Open the agent to log in again" hint shows only
for "not logged in", so it no longer shows while the probe can answer.
Codex's expired token stays "not logged in": nothing in OTTY renews it.

Tests seen failing first, against stubs (the new error case, an unused
`TrustQuestion`, an answer that always waits): the expired token test,
the screen test of the trust question, six of the seven trust module
tests and four probe tests (answered and panel read, marker moved, another
folder refused, a question that stays). The trust test "with no marker in
the options nothing is pressed" and the probe test "another question is
never answered" passed against the stubs, as they check what did not
change. The two app tests (the source row of `LoginExpired`, no login hint)
were written before the app code but not run before it; the second one
would have passed with only the new error case.

Checks: `cargo +nightly fmt`, `clippy --workspace --all-targets
--all-features -D warnings`, `cargo deny check` and one workspace test run
(5624 passed) pass. No coverage run.

Open: the exact text of today's Claude Code trust screen (whether the
folder is shown in full or from home, and the "Ready to code here?"
options) is from CodexBar and earlier notes, not from a real screen; the
run by hand on the owner's profile will show it. The spec's Decision 5
note ("an expired token is a failure of that source, and the profile says
'open the agent to log in again'") is overridden by Decision 12 and was
left as it is.
