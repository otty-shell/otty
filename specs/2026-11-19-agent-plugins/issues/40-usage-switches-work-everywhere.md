# 40 The usage switches work everywhere

Status: done
Blocked by: 39

## Why

Owner, 2026-11-23: "the switches between Money / Tokens and between 7,
30 and 1 day (Today) do not work well, check everywhere in Agents mode
that they work."

The period switch (Today, 7 days, 30 days) and the measure switch
(Tokens, Money) are the two drop-downs of the usage block (spec
2026-11-19, Decision 6). The block shows in three places:

| Place | Its choices | How a drop-down opens | What was broken |
| --- | --- | --- | --- |
| Page of a profile | `AgentsState::usage`, one for every profile, kept while OTTY runs | a menu of the mode at the pointer (`AgentsMenu::UsagePeriod`, `AgentsMenu::UsageMeasure`) | A press on Today, 7 days, 30 days, Tokens or Money only closed the menu. The frame's guard of open menus (`context_menu_guard` in `crates/app/otty/src/guards.rs`) did not list `PickPeriod` and `PickMeasure`, so it took them for a press outside the menu: it closed the menu and dropped the choice. The feature's own tests passed, since they call the reducer directly. |
| Statistics popup of a session | the popup's own `UsageChoice` while it is open | its list under the field | Nothing: the dialog guard lets every intent of the block pass. |
| Statistics popup of a project | the popup's own `UsageChoice` while it is open | its list under the field | Nothing, as for a session. |

No other place of Agents mode has these switches: the account pages,
All profiles, All accounts and the project page show fixed numbers (the
Money block of an account is this month, the Today column of All
profiles is today).

The same guard also took the pointer moving over the rows of the usage
table (`HoverUsageRow`) for a press, so moving the pointer over the
table under an open menu closed the menu.

Checked and found right, each now with a test:

- Today has 24 bars, one per hour, from 00:00 to 23:00, in the user's
  time zone. 7 and 30 days have one bar per day, today the last bar.
- The total under the title, the bars and the rows follow the period
  and the measure.
- Money writes dollars with the mark of an estimate. It is off, and the
  block shows Tokens, when the profile pays by a subscription or the
  prices are not known: the measure menu of a page says why, and so
  does the line under the title of a popup.
- The page's choices stay when another profile shows, and a popup's
  choices do not touch the page's.

## What was built

1. **The usage menus pass the frame.** `context_menu_guard` lets
   `PickPeriod`, `PickMeasure` and `HoverUsageRow` through to the open
   menu, as it does for the items of every other menu of Agents mode.
2. **Tests that press the switches.** Through the frame: every item of
   both usage menus passes, the pointer over the rows keeps a menu
   open, and Today and then Money picked in the menus of a profile page
   change the total shown under Usage. In the feature: each period of
   the page shows its bars, axis and total. Money and Tokens switch the
   total, the bars and the rows. The choices stay on another profile.
   The switches of the session popup and of the project popup change
   their bars and totals and leave the page's choices alone.

## Done when

- Tests first: the frame tests fail before the fix and pass after it.
  The feature tests pass and keep the rules of the block.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-23, done. One cause: the frame's guard of open menus did not
know the items of the two usage menus, so on a profile page a choice
closed the menu and changed nothing. The popups were not hit, since
their drop-downs open lists under the fields and the dialog guard lets
them pass. The numbers behind the switches were right. The new frame
test `every_item_of_the_agents_usage_menus_passes` presses every item as
the other menu tests do, and `a_period_and_a_measure_picked_in_their_menus_show_on_the_page`
goes from the press in the menu to the total on the page, so a guard
that drops them again fails a test.

Questions for the owner, left as they are:

- When Money is picked on a page and the next profile pays by a
  subscription, that page shows Tokens and says why only in the measure
  menu. Should the block say so too, as a popup does under its title?
- With a menu of the usage block open, a press on the other drop-down
  only closes the menu, as a press outside any menu does in OTTY, and a
  second press opens it. Should it open the other menu at once?
