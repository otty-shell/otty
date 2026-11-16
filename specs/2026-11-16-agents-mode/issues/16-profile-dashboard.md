# 16 The profile dashboard and the profile dialog in the new look

Status: ready-for-agent
Blocked by: 08, 09, 12, 14

## Why

Selecting a profile shows its dashboard: what the account costs in its
billing, the tokens spent, the profile's facts and where the numbers came
from (spec Decisions 6 and 9; canvas screens "Agent dashboard: Claude Code",
"Agent dashboard: Codex, credits", "Agent dashboard: API key, money spent",
"Add agent profile").

## What to do

1. The header: logo, profile name, status badge, plan and account; Refresh,
   "Edit", "⋯" (Hide; Remove for an added profile). A profile that is not
   logged in shows "Log in" there.
2. The left column: the billing block of spec Decision 9 (windows as bars
   with the reset time; credits with the bar only when the pool size is
   known; money today, 7 days, this month, the bars per day, "an estimate"
   with the price table date, and the budget bar when one is set), then
   "Tokens, last 30 days" with today, 7 and 30 days and a bar per day. The
   last good answer stays, greyed, with its time when a refresh fails (spec
   Decision 5).
3. The right column: "Profile" as a `KeyValueList`; "Top projects, 30 days"
   for windows and credits, "By model, this month" for an API key; "Where
   the numbers come from": each source in order with its state.
4. A block the agent does not support is left out.
5. The bars per day: a small kit component if the kit has none (bars on a
   baseline, today in the accent color, the value in the tooltip), drawn
   from tokens; check the dataviz rules of the design system.
6. The Add and Edit profile dialog in the look of the canvas: the agent as
   a `Select` drop-down with each agent's name and its provider as the note
   (not tabs: the list grows with every provider), the way to give the
   folder as a segmented choice, the folder field with its hint. Edit adds the program path. The budget and the admin key
   fields of an API key profile come with issue 17.
7. Remove what this replaces from the issue 05 and 06 drawing.

## Done when

- Tests, written first, cover the view models of each billing, the left-out
  blocks, the greyed last answer, and the dialog reducer changes.
- Run by hand: the steps the agent writes in its report.
- The checks of `AGENTS.md` pass.

## Comments
