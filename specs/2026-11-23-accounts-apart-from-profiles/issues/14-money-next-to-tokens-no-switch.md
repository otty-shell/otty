# 14 Money shows next to tokens; the Tokens / Money switch goes

Status: done
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

2026-11-24, done. The Tokens / Money switch is gone: `UsageMeasure`,
`OpenMeasureMenu`, `PickMeasure`, `AgentsMenu::UsageMeasure` and its
menu, the measure list of the popups (the popups keep one list, the
periods, `PeriodPicker`), the measure trigger of `menu_opened_by` and of
the dialog guard, and the text `agents_money_no_prices`, which nothing
used any more (all ten locales).

The usage block counts each row's tokens and money together
(`UsageGroup` holds both per point). The total is "400K tokens · ~$12.40
in 7 days" (new text `tpl_agents_tokens_and_money`): the money starts
with "~" when any of it is at API prices or unpriced, and the mark "at
API prices" with its tooltip stays under the table; exact money, the cost
the agent wrote, has neither. A bar is as tall as its tokens and its
tooltip says both, the lit part too. Every usage table (By project, By
model, By profile, the popups) has a Tokens and a Money column, both
right-aligned like numbers. A row whose money has no price at all shows
"—"; the cells carry no "~", the mark under the table covers them. Before
the list prices are read the money is "…" in the total, the bars, the
cells, and the Cost of Project statistics (it was "—").

Column minimums: name 120, Tokens 72, Money 88, Share 56, 336 px in all.
The smallest page (`size::SCREEN_MIN` less the page padding) leaves the
table 552 px, and the popups are wider (`size::DIALOG_WIDE`), so no table
scrolls sideways.

Tests: no measure switch on a page or in a popup (Tokens and Money are
column titles under the tabs), the total with tokens and money, "~" and
the mark only for an estimate, Tokens and Money columns with "—" for a
model with no price and the money aligned to its column's end, bars as
tall as their tokens with both in the tooltip, "…" before the prices, the
table fits the smallest screen and the dialog, and the period menu still
passes the frame. The tests and the code of each module were written in
one pass, not test first; to see them fail, three rules were broken for a
run (bars by money, no "~", a Money column 400 px at least): 10 tests
failed, and they passed again once the rules were back. Docs:
Overview.mdx; spec Decision 3 has a "Built as" line.
