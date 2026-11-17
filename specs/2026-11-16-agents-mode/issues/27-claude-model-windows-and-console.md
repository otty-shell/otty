# 27 Claude: every model window, Console profiles billed by money

Status: ready-for-agent
Blocked by: 26

## Why

Spec Decision 11. The owner's subscription shows a Fable weekly limit in
Claude, but OTTY draws only the windows it knows by name. And the owner's
second profile (`~/.claude-semrush`) is logged in to a Console organization:
its `oauthAccount.billingType` is `prepaid` (the main profile says
`stripe_subscription`), it has no subscription windows, and OTTY shows
neither limits nor money for it, while VS Code shows the dollars spent.

## What to do

1. The usage answer (`claude_code/usage.rs`): every model-specific weekly
   window becomes a bar, named after its model: the `seven_day_<model>` keys
   (`seven_day_opus`, `seven_day_sonnet`, and any other such key, such as
   Fable) and the entries of `limits[]` with `weekly_scoped`, as CodexBar
   maps them (read its Claude usage code, MIT,
   https://github.com/steipete/CodexBar, with `gh api` or WebFetch). Keys
   that are not windows (`seven_day_oauth_apps` as a fallback,
   `seven_day_routines`, `seven_day_cowork`, `extra_usage`, `iguana_necktie`,
   `cedar_ember`) keep the meaning CodexBar gives them or are skipped. The
   `/usage` screen reader (issue 10) also reads every "Current week
   (<model>)" block. Names: "7 days · Fable", like the others.
2. `detect` for Claude Code: `billingType` `prepaid` (and any other Console
   billing type CodexBar or Claude Code treats as API billing; check) with no
   API key in the files is an API key profile (spec Decision 9): no limits
   read, the money estimate from the session files. Never read or keep a
   token for this.
3. The header of such a profile says "Anthropic API, pay per token" as for
   other API key profiles.

## Done when

- Tests, written first, cover: an answer with Opus, Sonnet and a third model
  key, an answer with `limits[]` weekly entries, a key that is not a window;
  the `/usage` screen with three week blocks; a `.claude.json` with
  `billingType` `prepaid` and one with `stripe_subscription`.
- Test data is made up; no real file or answer is copied.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
