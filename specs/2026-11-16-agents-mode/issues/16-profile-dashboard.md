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

### 2026-11-16 (coordinator)

From issue 11: the billing kind of a profile comes from
`ProfileStatus::Connected { billing }`. For an API key profile the app does
not read limits at all (its sources would only say "not available"); the
billing block shows the money of issue 12 instead, and the header says
"Anthropic API, pay per token" (or the OpenAI one) where a plan would be.

### 2026-11-16

Built in `otty-app-agents`, the kit and `otty-agents`, tests first and seen
failing. Pressing a profile now shows its dashboard
(`model/dashboard.rs`, `view/dashboard.rs`): the logo, the name, a
`StatusBadge`, the plan and the account (an API key profile says
"Anthropic API, pay per token" or the OpenAI one, from the new
`AgentDescriptor::provider`), then Refresh, Log in while not logged in,
Edit and "⋯", a kit `Menu` at the pointer with Hide and, for an added
profile, Remove. On the left the billing block: the usage windows as bars
with the percent and the reset, the extra usage in money (a bar only with a
limit), or the credits (used of the pool and the bar only when both are
known, what is left, when the pool renews); for an API key the Spending
block of issue 12: today, 7 days and this month with their tokens, a bar per
day, "an estimate at the list prices of <date>", and the requests of the
month with no price. Then "Tokens, last 30 days": today, 7 and 30 days
(in · cached · out) and a bar per day. On the right: Profile as a
`KeyValueList` (account, plan, config folder with `~`, the program the
agent would run, "N in M projects", last used and where), "Top projects, 30
days" (five, with the sessions of the profile in each) or "By model, this
month" as a `DataTable`, and "Where the numbers come from": every limit
source of the agent in order from the latest read, as answered, not
needed, login expired, not available (with the reason) or failed (with the
error); an API key has one row, the session files at list prices.
`LoadedLimits` got `sources`, the names of all the agent's sources, so the
ones after the answer show as not needed. A greyed last good answer keeps
its time in "From <source>, <age>" (limits) or "Last read <age>" (tokens,
money); the bars turn muted. A block the agent can not give is left out.
API key profiles no longer have their limits read. The old details screen,
its limits block and its two token facts are gone, with their five unused
texts.

The dialog: the agent is a drop-down, a new kit `NoteSelect` (a field with
the picked option and its note, a list under it with each agent and its
provider, built on the kit's floating list), opened and closed by the new
`AgentListToggled`; a pick closes it. The way to give the folder stays a
`segmented`; the path field is labelled with the picked way and a typed
path has its own hint. Other kit additions, each in the gallery:
`DayBars` (bars on a baseline, today in the accent color, the value in a
tooltip, first day and Today under them, muted variant) and
`ProgressBar::fill_width` and `ProgressBar::muted`. New texts are in all
ten languages.

Deviations: `NoteSelect` is a new component, not an option of `select`:
iced's pick list draws only one text per option. The reset and renewal
times stay relative ("resets in 3 h"), as before, without the clock time
of the canvas, which would need the user's time zone. Codex's credits
answer has no numbers for today, 7 days or this month, so those three
credit totals of the canvas are not drawn. The source names are the short
ones the sources give ("usage request"), not the longer canvas wording. A
day with nothing spent has no bar; a day with a little has a 2px sliver.
The budget bar and the admin key are left to issue 17.
