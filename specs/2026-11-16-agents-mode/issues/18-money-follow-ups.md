# 18 Follow-ups of the money work

Status: ready-for-agent
Blocked by: 17

## Why

Issue 17 left three small choices that the coordinator decided on
2026-11-16.

## What to do

1. **The admin key variable is read through the user's login shell**, the
   same way programs are found (issue 05), so a variable set in `.zshrc`
   is seen when OTTY was started from the desktop. The value is read on the
   read thread, kept only for the request and never logged.
2. **The number in the left panel row of an API key profile is the
   profile's own estimate** ("$12.40 today"), not the organization's bill,
   because the row is about this profile. The dashboard keeps the bill in
   its Spending block.
3. **"By model, this month" says it is the estimate** while the bill
   answers: "By model, this month (estimate at list prices)".

The `anthropic-version` header stays `2023-06-01`.

## Done when

- Tests, written first, cover each point.
- The checks of `AGENTS.md` pass.

## Comments
