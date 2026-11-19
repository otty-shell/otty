# 13 Anthropic plugin, Claude Code part: the `/usage` probe, embedded; core Claude Code deleted

Status: ready-for-agent
Blocked by: 06, 08, 12

## Why

Spec Decisions 6, 7 and 10. Claude Code shows some limits only on its
`/usage` screen. Today OTTY starts `claude` in a hidden terminal, answers
the trust question, reads the screen (`usage_probe.rs`,
`usage_screen.rs`, `trust_question.rs`, `window_id.rs`) and then deletes
the session files the probe left (`probe_sessions.rs`). In a plugin this
is a hidden terminal plus a declared write of the `claude-code` part.

## What to build

1. A limit source in the `claude-code` part through
   `terminal.hidden-terminal`: start `claude` with the arguments of today
   (declared in the part's `[[agent.permissions.terminal]]`), wait for
   the screen, answer the trust question, send `/usage`, parse the screen
   text into limit windows; all timing and text in the plugin.
2. Probe cleanup: a `write` permission of the part for only the `*.jsonl`
   files of the project folder of OTTY's probe working folder (spec
   "Permissions" wording "Deletes files in …"); remove them with
   `files.remove`, then the folder when empty, as `probe_sessions.rs`
   does.
3. Pacing stays in the host (issue 06): at most once in five minutes.
4. Embed the built copy of the Anthropic plugin in
   `assets/plugins/anthropic/` (issue 08); built from `plugins/anthropic/`
   by the script of issue 11 while the plugin lives in this repository.
5. Delete `crates/agents/src/claude_code/` except `usage.rs`,
   `usage_answer.rs` and `cost_report.rs`, which move to
   `account_numbers/` until issue 15; delete the built-in entry. Only
   after a full parity run (tokens, money, limits) on real files.

## Done when

- Tests first: screen parsing on made-up screen texts (from today's
  tests of `usage_screen.rs`); the trust question answered; the probe
  sequence with a mock terminal; cleanup removes only the probe's files
  and refuses any other path.
- Parity "equal" recorded; the core folder is gone except the moved
  files; the app shows Claude Code from the embedded Anthropic plugin.
- The checks of `AGENTS.md` pass; the plugin's own tests pass.

## Owner checks by hand

Limits of the Claude login appear on the account page; no probe session
is left in `~/.claude/projects/`; Profile info says "Read by: Anthropic
<version>".

## Comments
