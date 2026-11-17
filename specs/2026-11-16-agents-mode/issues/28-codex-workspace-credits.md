# 28 Codex workspace credits

Status: ready-for-agent
Blocked by: 27

## Why

Spec Decision 11. The owner's Codex account is on an enterprise workspace
with credits (`plan_type` `enterprise_cbp_usage_based`). The Codex app shows
the workspace's credit pool (about 80 000) and how much was used; OTTY shows
only "Credits: available", because the `credits` part of the answer has no
balance (issue 19). CodexBar reads these numbers elsewhere (read on
2026-11-17 in `Sources/CodexBarCore/Providers/Codex/CodexOAuth/
CodexOAuthUsageFetcher.swift` and its tests `CodexSpendControlsMonthlyUsage
Tests.swift`, `CodexWorkspaceBalanceTests.swift`, `CodexOAuthCreditLimitTests
.swift`):

- the usage answer (`wham/usage`) carries an individual limit at its root
  (`individual_limit`), under `rate_limit.individual_limit`, or under
  `spend_control.individual_limit` (team and enterprise workspaces), in that
  order: `limit`, `used`, `remaining_percent`, `resets_at` (numbers may be
  strings);
- `GET <chatgpt base>/backend-api/accounts/<account id>/spend-controls/
  current-user/monthly-usage` gives the user's monthly usage against the
  workspace limit;
- `GET <chatgpt base>/backend-api/accounts/<account id>/remaining_balance`
  gives the workspace's `balance`.

## What to do

1. Read CodexBar's fetcher and those three test files (MIT; `gh api
   repos/steipete/CodexBar/contents/<path>` or WebFetch) and follow them:
   field names, number forms, precedence, which request is sent when, how a
   refusal of the extra requests is treated (they must never break the main
   usage read). A file that follows CodexBar closely says so with a link.
2. Map the result to `Credits` (issue 11 / 19): pool = the limit, used, left
   = limit − used or the balance, renews at = `resets_at`; the dashboard
   then shows "used of pool" with the bar, left, and the reset, and the left
   panel row "N left". Only numbers that came are shown (spec Decision 9).
3. The extra requests go through the existing `HttpsGet` with the same token
   and `ChatGPT-Account-Id` header as the usage request; they run only when
   the usage answer says credits are on and has no numbers, and at most once
   per limits read.
4. Tests use a mocked request and made-up answers in the shapes of
   CodexBar's tests; no real request, no real token.

## Done when

- Tests, written first, cover: the three places of the individual limit and
  their order; string numbers; the monthly usage answer; the remaining
  balance answer; a refused extra request leaves "Credits: available"; the
  dashboard and the row with a pool and a used part.
- Run by hand: the owner's Codex profile shows the pool and the used part as
  the Codex app does.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
