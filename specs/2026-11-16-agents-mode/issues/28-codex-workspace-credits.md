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

### 2026-11-17

Built in `otty-agents`. The app needed no change, only two tests. I read
CodexBar's fetcher, the three test files, `CodexSpendControlLimitMapping`,
`CodexSpendControlNumber`, `CodexSpendControlsMonthlyUsage`,
`CodexProviderDescriptor` (the order of the extra requests and their gates)
and the RPC shapes in `UsageFetcher.swift`. The copies stayed in `target/`.

The individual limit (`codex/individual_limit.rs`, which follows CodexBar
and links to it) is read in the same way by all three sources, so they
give the same numbers:

- usage answer: `individual_limit` at the root, else under `rate_limit`,
  else under `spend_control`. Each one may also be spelled `individualLimit`,
  and `spendControl` is read too. The first place that holds an object wins,
  as in CodexBar's `resolvedIndividualLimit`.
- `app-server`: `rateLimits.individualLimit` (or `individual_limit`).
- session file: `rate_limits.individual_limit`.
- Fields: `limit`, `used`, `remaining_percent` / `remainingPercent`, and
  `resets_at` / `resetsAt` / `reset_at`. Any number may come as text (a new
  `codex_number`, which the balance now uses too). It must be finite.
- Mapping to `Credits`: pool = `limit`, which must be above 0. Used =
  `used`; else pool × (100 − `remaining_percent`) / 100; else 0. Left =
  pool − used, never below 0. Renews at = the reset, when it is above 0.
  The pool wins over the balance of `credits` (CodexBar shows the limit next
  to a balance of `"0"`), and it also wins over `unlimited`.

The extra requests (`codex/workspace_credits.rs`, which follows CodexBar
and links to it) are made only by the usage request source, and only when
the answer's credits are on with no number (`Credits::is_only_available`).
They are sent through the same `HttpsGet` with exactly the usage request's
headers: the token and `ChatGPT-Account-Id`. They go only to a server with
`/backend-api` (`backend_api_url` in `base_url.rs`). The account id is
percent-encoded as one part of the path. Each request is sent at most once:

1. `accounts/<id>/spend-controls/current-user/monthly-usage`, only when the
   answer has a `spend_control` key (even `null`) and the plan is not
   `guest`, `free`, `go`, `plus` or `pro`. Pool = `effective_monthly_limit
   .limit` (above 0). Used = `current_month_usage`, never below 0; a missing
   value is 0. Left = pool − used. An `enforcement_mode` of `none`, `off`,
   `disabled` or `no_limit` (any letter case) means there is no pool. A
   number or a mode that can not be read also gives no pool.
2. `accounts/<id>/remaining_balance`, when step 1 gave no pool. Left =
   `balance`, never below 0. There is no pool, so there is no bar.

Any failure of these requests (no answer, a refusal, another status, a body
that is not JSON or has no number) leaves "Credits: Available". It is not a
source failure. Nothing is logged.

Deviations from CodexBar:

- CodexBar asks for the balance first and then the monthly usage, and may
  send both. OTTY asks for the monthly usage first and asks for the balance
  only when no pool came. The pool's left part wins anyway, so this saves a
  request.
- The account id comes only from `auth.json`, so no header is sent that the
  usage request did not send. CodexBar falls back to the answer's
  `account_id`. When the answer names another account, OTTY sends neither
  request. CodexBar skips only the balance in that case, and it also asks
  for the balance only when the answer names an account.
- `rateLimitsByLimitId` of `app-server` is not searched for a limit (CodexBar
  also looks there, sorted by name).
- Windows still win over credits (issue 11). A team answer with both a
  window and an individual limit shows only the windows.

Tests seen failing first: all the new `otty-agents` tests (run against
stubs that returned nothing), plus the two old usage tests whose answer now
also asks for the balance. The two app tests (the dashboard with a pool,
the used part, the bar and the renewal; the left panel row "92 239 left")
passed at once, because the app already drew a pool. They pin that
behaviour.

Checks: fmt, clippy, deny and one workspace test run (5609 passed) pass.
Coverage was not run.

To check by hand: open the owner's Codex profile. It should show
"Credits: N of M used" with a bar, "K left · Resets …" when the answer or
the monthly usage has a pool, or "K left" from the workspace balance. If it
still says "Available", none of the three places, nor the two requests, gave
a number for this account.
