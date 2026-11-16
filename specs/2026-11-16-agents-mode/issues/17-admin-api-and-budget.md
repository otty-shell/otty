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
