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

### 2026-11-16

Built in `otty-agents`, tests first and seen failing. The new
`src/billing.rs` holds the billing: `Billing` is a plain enum of
`UsageWindows` (the windows and the `ExtraUsage`: spent, limit and the
currency code, in whole units, not cents), `Credits` (left, used, pool,
renews at, unlimited; every number optional) and `ApiKey` with no numbers.
`Limits` is now the plan and one `Billing`. `UsageWindows::most_used` gives
the window with the highest percent (the first of equal ones), and
`Credits::left` the credits left, for the left panel number.

`detect` tells the billing in a new field of `ProfileStatus::Connected`,
`billing: Option<BillingKind>` (`UsageWindows`, `Credits`, `ApiKey`, the
same as `Billing::kind()`). A separate kind with no numbers keeps
`ProfileStatus` comparable with `Eq`. Claude Code is an API key profile
with a non-empty `env.ANTHROPIC_API_KEY` or `apiKeyHelper` in
`<config>/settings.json` or a `primaryApiKey` in its `.claude.json`; it then
counts as connected even with no login token, and it shows no
subscription plan. Otherwise it is a subscription. Codex is an API key
profile by `auth_mode`, else by a non-null `OPENAI_API_KEY`; else `None`,
and the limits answer tells. The keys are read only as "there and not
empty" and dropped at once.

Sources: Claude gives windows and the extra usage (in money, while turned
on); an answer with neither is still "unexpected". Codex gives windows when
the answer has them (its credits are then dropped), credits when it has
only credits (a balance with no total has no "used" and no pool; `has_credits`
with no balance gives credits with no numbers), and "unexpected answer"
with neither, so the next source of issue 08 is tried. The coordinator's
note is done: a missing, empty or unreadable `.credentials.json`, or one
with a blank token, is "not available: no login token in
.credentials.json"; an expired token stays "not logged in".

Deviation: Codex's `auth_mode` wins over a key in the file, because older
Codex versions kept an `OPENAI_API_KEY` next to a ChatGPT login and Codex
itself reads the mode first. In the app only what the shape forced
changed: the bars come from the windows billing, the credits text from the
credits billing; the old "Extra usage" bar is gone until issue 16 draws the
extra usage in money. The app does not use the detected billing yet.
