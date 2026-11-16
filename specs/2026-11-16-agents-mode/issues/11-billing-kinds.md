# 11 Three kinds of billing in `otty-agents`

Status: ready-for-agent
Blocked by: 07

## Why

An account is paid for by usage windows, by a pool of credits or by an API
key charged per token (spec Decision 9). The dashboard and the number in the
left panel follow the billing, so the data must say which one it is.
Issue 07 built `Limits` as windows and credits only.

## What to do

1. Change `Limits` so it holds the plan name and one billing with its
   numbers, every number optional:
   - usage windows: the windows of today (name, percent used, reset time),
     and the extra usage when the plan has it (spent and limit in money);
   - credits: credits left, credits used, the pool size, when it renews,
     unlimited;
   - API key: no numbers of its own (the money comes from `Spend`, issues 09
     and 12), and the budget is the profile's (issue 17).
   Keep it a plain enum with plain structs; no trait for it.
2. A method that gives the one short text for the left panel row of spec
   Decision 6 is the view's job; here give what it needs: "the most used
   window" for windows, "credits left" for credits.
3. `detect` tells the billing of a profile from its files:
   - Claude Code: an API key in the profile's `settings.json`
     (`env.ANTHROPIC_API_KEY`, or an `apiKeyHelper`) or a `primaryApiKey` in
     `.claude.json` makes it an API key profile; else a subscription.
   - Codex: `auth_mode` of an API key, or a non-null `OPENAI_API_KEY` in
     `auth.json`, makes it an API key profile; else the billing comes from
     the limits answer (windows, or credits when the answer has only
     credits).
   The key itself is only checked, never kept or logged.
4. Move the issue 07 sources onto the new shape: Claude windows and extra
   usage; Codex windows, or credits when the answer has no windows.
5. The app keeps working with the old drawing until issue 16; change only
   what the new shape forces there.

## Done when

- Tests, written first, cover: each billing built from made-up answers;
  a Codex answer with credits and no windows is credits; a Codex answer with
  a balance and no total has no "used" and no pool; API key detection for
  both agents from made-up files; the most used window.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16 (coordinator)

From issue 07: a missing or empty `.credentials.json` makes the Claude usage
request "not available" (the login can be kept elsewhere, as on the owner's
second profile and on macOS), not "not logged in", so the profile does not
say "open the agent to log in again" while it is logged in. The
`User-Agent` stays `otty/<crate version>`.
