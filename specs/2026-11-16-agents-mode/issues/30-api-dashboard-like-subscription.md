# 30 The API key dashboard looks like the subscription one

Status: ready-for-agent
Blocked by: 29

## Why

The owner expected the dashboard of an API key profile to look exactly like
a subscription's, with money in place of the usage bars. Today it has its
own layout: a Spending block with a chart and notes, and "By model, this
month" in place of the top projects (spec Decision 12).

## What to do

1. The Usage block of an API key profile: title "Usage", "Updated …" under
   it (as for a subscription), then the money as rows in the same style as
   the window rows: "Today  $12.40", "Last 7 days  $63.10", "This month
   $184.75", and the budget row with its bar when a budget is set. No chart
   and no note inside the block ("an estimate", "from the bill" and the
   price date go only to "Where the numbers come from", as the source names
   and their tooltips). Requests with no price: one muted row at the end of
   the block when there are any.
2. The rest of the dashboard is the subscription's: "Tokens, last 30 days"
   with its bars, Profile, "Top projects, 30 days", "Where the numbers come
   from". "By model, this month" is gone from the dashboard; keep the
   per-model money in `otty-agents` only if something else still uses it,
   else remove what becomes unused.
3. The left panel number stays "$X today".

## Done when

- Tests, written first, cover: the API key Usage rows with and without a
  budget, the no-price row, the same right column as a subscription, no
  chart in the Usage block.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments
