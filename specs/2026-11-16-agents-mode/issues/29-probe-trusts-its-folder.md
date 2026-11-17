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
