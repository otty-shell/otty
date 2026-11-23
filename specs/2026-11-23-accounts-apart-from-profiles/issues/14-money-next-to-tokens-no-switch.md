# 14 Money shows next to tokens; the Tokens / Money switch goes

Status: ready
Blocked by: 13

## Why

Owner, 2026-11-24: since issue 01 every profile has money (the cost the
agent wrote, or its tokens at API prices), and Money is off only until
the price list is read. So the Tokens / Money switch no longer chooses
anything, it shows the same rows in two units. Show money as a column next
to the tokens and drop the switch.

## What to build

1. **No measure switch.** The Tokens / Money drop-down leaves the usage
   block of the profile page and the session and project statistics
   popups (`UsageMeasure`, `PickMeasure`, its menu and guard trigger in
   `crates/app/otty/src/guards.rs` `menu_opened_by`, its words). The
   period switch (Today, 7 days, 30 days) stays.
2. **The total says both**: "400K tokens · ~$12.40", the money with the
   "at API prices" mark (and its tooltip) when any of it is estimated, no
   mark when it is all a cost the agent wrote. While the prices are not
   read yet the money is "…".
3. **Tables show both**: By project, By model, and the tables of the
   statistics popups get a Tokens column and a Money column side by side.
   A model with no price shows "—" in Money. Columns keep the kit table
   look, money right-aligned like numbers, and the tables still fit the
   smallest screen (`size::SCREEN_MIN`, see issues 38 and 39 of spec
   2026-11-19-agent-plugins: no table may scroll sideways inside a page
   that does), so check the column minimums.
4. **Bars by tokens.** The bars of the usage block (hours of Today, days
   of 7 and 30 days) are as tall as their tokens. The tooltip of a bar
   says both, "120K tokens · ~$3.10". Bars never follow money, since a
   model with no price would drop out of them.
5. The project page's Cost line and anything else that showed money only
   when Money was picked shows it always. Words nothing uses any more go
   from all ten locales. Docs (`docs/public/Agents/Overview.mdx`) follow.

## Done when

- Tests first: no measure switch on the profile page or in the popups;
  the total shows tokens and money with the mark only for an estimate;
  every usage table has Tokens and Money columns and "—" for a model with
  no price; bars are as tall as their tokens whatever the money; a bar's
  tooltip has both; before prices the money is "…"; the tables fit the
  smallest screen.
- The checks of `AGENTS.md` pass.

## Comments
