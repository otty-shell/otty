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

### 2026-11-17

Built in `otty-app-agents`, `otty-agents` and `otty-app-i18n`, tests first.

Usage block of an API key: under "Usage" and "Updated …" the money is
rows drawn by the same piece as the usage windows (`blocks::usage_row`,
moved out of the limits view; `WindowViewModel` is now
`UsageRowViewModel`, since a row is a window or money): "Today", "Last 7
days", "This month", each with its amount on the right and no bar, then
"Your monthly budget" with "$X of $Y" and its bar when a budget is set.
Requests with no price are one muted line at the end; a failed count
stays in the error tone, as for the limits. The bars per day, the token
lines under the money and the notes are gone from the block, the budget
note ("nothing stops at the budget") too, since Decision 12 wants no note
in the block. What the money is now lives in the tooltips of "Where the
numbers come from": the estimate that answered says "Tokens from the
session files at the list prices of <date>, an estimate", the cost report
that answered says "From the bill, for the whole organization".

Right column: "Top projects, 30 days" for every billing; the by model
table is gone (`TopTableViewModel`, `ModelRowViewModel`, `by_model`), so
the dashboard keeps `Option<Vec<TopProjectRowViewModel>>`. The left panel
still says "$X today" from the estimate.

Removed as unused: the texts `agents_by_model`, `agents_by_model_estimate`,
`agents_column_model`, `agents_column_input`, `agents_column_output`,
`agents_column_cost`, `agents_no_price`, `agents_unnamed_model` and
`agents_budget_note` in all ten languages; `BudgetViewModel`; in
`otty-agents`, `MoneyEstimate::this_month_by_model`, `ModelMoney`,
`MoneyEstimate::last_30_days_by_day` and `DayMoney` (now private: it only
adds up today and 7 days). `Bill::last_30_days_by_day` is kept for the
tests of the cost report only. `Spend::this_month_by_model` stays: the
estimate of this month is made from it.

Tests seen failing first: the dashboard tests of the top projects of an
API key, of the source tooltips and of no bars per day in the Usage block
(against the old code), then the money rows tests and the view tests of
the block (against a stub that gave no rows). The `otty-agents` tests
were only changed to stop reading what was removed; they were not run
failing first.

Checks: fmt, clippy, deny and one workspace test run (5623 passed) pass.
Coverage was not measured.
