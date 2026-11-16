# 17 The Anthropic Admin API cost report and the budget

Status: ready-for-agent
Blocked by: 12, 16

## Why

The estimate of issue 12 is not the bill. With an admin key the real cost of
an Anthropic organization can be read (spec Decision 9). The user also wants
to see spending against a monthly budget of their own.

## What to do

1. In Edit of an API key profile: "Admin key" as a path to a file that holds
   the key, or the name of an environment variable; and "Monthly budget" in
   dollars. `agents.json` stores the path or the name and the budget, never
   the key. The key is read when the cost is read, kept only for that
   request, never logged.
2. A money source before the estimate, for Claude Code API key profiles:
   `GET /v1/organizations/cost_report` of the Anthropic Admin API with that
   key. Read the Anthropic docs for the headers, the paging and the fields,
   and CodexBar's Claude admin code for how it groups the report. Its answer
   gives today, 7 days, this month and per day; the block then says "from
   the bill" instead of "an estimate". With no admin key the source is "not
   available: no admin key", and the estimate answers.
3. The budget bar of the dashboard: spent this month of the budget. It
   never stops or warns beyond the bar.

## Done when

- Tests, written first with a mocked request, cover: a report; paging; a
  refused key; no key; a key file that is missing; the budget saved and
  loaded; the key never in `agents.json`.
- No test reads a real key or calls the real API.
- The checks of `AGENTS.md` pass.

## Comments

### 2026-11-16

Built tests first with a mocked request and made-up key files; no test
reads a real key or the real environment, and none reaches the network.
`agents.json` keeps, per profile (default or added), an optional
`"admin_key": { "file": "<path>" }` or `{ "env_var": "<name>" }` and an
optional `"monthly_budget": 300.0` in dollars; older files load unchanged,
and the key itself is never written. In `otty-agents`, `AdminKeyRef` reads
the key from its file or from a variable the app passes in; the key lives
in an `AdminKey` that can not be copied and prints as hidden, and it is
dropped after the request. `Agent::bill_source` gives Claude Code's
`CostReport` (Codex gives none): `GET /v1/organizations/cost_report` with
`x-api-key`, `anthropic-version: 2023-06-01` and `Accept:
application/json`, `starting_at` 30 UTC days before today,
`ending_at` the end of today, `bucket_width=1d`, `limit=31`, then
`page=<next_page>` while `has_more` is true (ten pages at most). The
amounts are cents as decimal text, summed per UTC day into a `Bill`
(today, 7 days, this month, 30 bars). 401 and 403 are the new
`SourceError::KeyRefused`. In the app the bill of a connected Claude Code
API key profile with an admin key is read with the profiles, on Refresh and
on the five-minute tick (`otty-app-agents-io::load_bill`). When the latest
read answered, the Spending block shows the bill, says "From the bill, for
the whole organization", the left panel number takes today from it, and
the sources list shows the cost report as answered and the estimate as not
needed; else the estimate answers and the cost report row says "not
available: no admin key" or why it failed. A Codex API key profile lists
only the estimate. The budget bar ("Your monthly budget", "$X of $Y", a
bar and a note that nothing stops) shows the money of this month from
whichever source answered. Edit of an API key profile has "Admin key" (File
or Environment variable and the value, Claude Code only) and "Monthly
budget, $"; a relative path, a wrong variable name or an amount that is not
dollars blocks Save. New texts are in all ten languages.

Choices and deviations: CodexBar groups the report by `description` to list
what the money was spent on; OTTY shows only the money of each day, so it
groups nothing. The bill's days are UTC days, as the report counts them,
while the estimate uses the user's days. "By model, this month" stays from
the estimate, since the bill has no tokens per model of this profile. A day
whose costs add up below zero counts as nothing, and a cost that is not in
USD or can not be read is skipped. The variable is read from OTTY's own
environment, so an app started from a desktop launcher sees only the
variables that launcher passes. The shared name-and-bar pieces of the
limits block moved to `view/blocks.rs` so the budget bar uses them too.
