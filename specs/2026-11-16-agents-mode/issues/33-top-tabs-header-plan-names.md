# 33 Top by project or by model; no plan in the header; plan names as in the data

Status: ready-for-agent
Blocked by: 32

## Why

The owner's fourth round of changes (spec Decision 13, 2026-11-17):

- The dashboard has no table by model since issue 30. The owner wants it
  back for every profile, next to the top projects, as tabs.
- The header repeats the plan and the account after the profile name
  ("Max 20x · user@example.com"). Both are already in the Profile list, so
  the header does not need them.
- Some plan names carry words OTTY made up: "Enterprise, usage-based" for
  Codex `enterprise_cbp_usage_based`, and "Anthropic API, pay per token" /
  "OpenAI API, pay per token" for API key profiles. The owner wants
  "Enterprise" and "Anthropic API": values as the data gives them, with no
  explanations of our own.

## What to do

1. **Tabs in the top section.** The section "Top projects, 30 days" gets
   two tabs, "By project" (first, chosen at start) and "By model", for
   every profile and every billing. Use the kit's `Segmented` control (or
   whatever the design system and the kit use for a small tab switch inside
   a screen; say which and why). Rename the section title so it fits both
   tabs, for example "Top, last 30 days"; keep it short and say what you
   chose.
   - By project: the table of today, unchanged.
   - By model: one row per model, the most tokens first, at most as many
     rows as By project: the model name as the data gives it, the number of
     requests, the tokens written short like the By project table. A model
     with no name in the data is one row "Unknown model". Same 30 days as
     the Tokens block and By project: add what `otty-agents` needs to give
     the models of the last 30 days (`Spend` already keeps the days by
     model; `this_month_by_model` stays for the money estimate).
   - No money column: the table is tokens for every billing.
   - The chosen tab is kept in the mode's state while OTTY runs, the same
     for all profiles; it is not saved to disk.
   - A tab with nothing to show shows the same empty text the section
     shows today when there are no projects; when neither tab has rows the
     section is left out, as today.
2. **Header.** The dashboard header shows the logo, the name and the
   buttons only: remove the plan and the account after the name (the
   `subtitle` of `DashboardViewModel` and its view). The Profile list keeps
   Account and Plan. Remove what becomes unused; the kit `HeadingRow` keeps
   its note option only if something else uses it, else remove it with its
   tests and gallery entry.
3. **Plan names from the data.**
   - Codex: `enterprise_cbp_usage_based` is "Enterprise". Check every name
     in `plan_name` of `crates/agents/src/codex/rate_limits.rs` against how
     Codex itself names the plan (the open source `openai/codex`, the plan
     type of its protocol and what its `/status` prints; reading the web is
     fine, never run anything against the real `~/.codex`). Keep a name
     only when Codex shows it; else the code in words, as for unknown
     codes today.
   - Claude Code: `plan_name` in `crates/agents/src/claude_code/plan.rs`
     already gives what Claude Code shows ("Max 20x"); check nothing is
     added.
   - API key profile: the plan is "<provider> API" ("Anthropic API",
     "OpenAI API"), no "pay per token". Rename the text key to match.
   - Look through the rest of the dashboard and the left panel for other
     values OTTY dresses up with its own words. Do not change them; list
     them in your report so the owner can decide.
4. All ten locales for every new or changed text.

## Done when

- Tests, written first and seen failing, cover: the two tabs and switching
  between them; the By model rows (order, the limit, "Unknown model", the
  30 days, not the calendar month); the tab kept when another profile is
  selected; an empty tab; the header with no plan and no account; the
  Profile list still with both; the Codex enterprise plan named
  "Enterprise"; the API key plan "Anthropic API" with no more words.
- Run by hand: open a subscription profile and an API key profile; switch
  the tabs; the header has only the name; the Profile list shows the plan
  "Enterprise" for the Codex enterprise profile.
- The checks of `AGENTS.md` pass (no coverage run).

## Comments

2026-11-17: Built as asked.

- **Tabs.** The section is now "Top, last 30 days" (`Key::AgentsTopTitle`;
  the same shape as "Tokens, last 30 days"). The tabs are the kit's
  `segmented`, at the right end of the title line: it is the kit's control
  for picking one of two to four short options inside a screen (the profile
  dialog uses it too); `inspector_tabs` is for the sections of the
  inspector and draws a line under the whole row, too heavy for one block.
  The chosen tab is `TopTab` in the mode's state (`AgentsIntent::ShowTop`),
  By project at start, the same for every profile, not saved. By model
  lists at most 5 models, as By project, with Model, Requests (in groups of
  three digits, like the credits) and Tokens (short); a model with no name
  is "Unknown model". `otty-agents` got `Spend::last_30_days_by_model`, the
  same days as `last_30_days`; `this_month_by_model` stays for the money
  and shares the adding up with it. The section is left out when neither
  tab has a row.
- **Empty tab: a deviation.** The section had no empty text before (it
  was only left out), so a tab with nothing to list shows a new muted line,
  "Nothing to show" (`Key::AgentsTopEmpty`), in place of the table.
- **Header.** `subtitle` is gone from `DashboardViewModel` and the header
  shows the logo, the name and the buttons. Nothing else used the note of
  the kit's `HeadingRow`, so `note`, `same_baseline`, their tests and the
  note in the gallery entry are removed, and so is
  `fonts::ui_baseline_below_middle`, added in issue 32 only for that note.
- **Plan names.** Codex `/status` names a plan with
  `SubscriptionDisplay::Status` in `codex-rs/tui/src/subscription.rs`: free
  "Free", go "Go", plus "Plus", pro "Pro 200", prolite "Pro 100", promax
  "Pro 500", team and self_serve_business_usage_based "Business",
  self_serve_business_prolite "Business Premium", business, enterprise,
  ent26 and enterprise_cbp_usage_based "Enterprise",
  enterprise_cbp_automation "Enterprise (Automation)", edu "Edu", edu_plus
  "Edu Plus", edu_pro "Edu Pro", sci "Academic Researchers", sci_plus
  "Academic Researchers Plus". Kept by the rule of this issue:
  enterprise_cbp_usage_based is "Enterprise"; plus, enterprise, edu and
  free keep their names (the code in words gives the same). "Pro Lite" for
  prolite is not a name Codex shows, so it is now the code in words,
  "Prolite". pro, team and business are their codes in words, "Pro",
  "Team", "Business", although Codex shows them as "Pro 200", "Business"
  and "Enterprise": the owner may want Codex's names there. Claude Code's
  `plan_name` adds nothing ("Max 20x" from `max` and the tier). An API key
  plan is "{provider} API" (`Key::TplAgentsProviderApi`, was
  `TplAgentsPayPerToken`), "Anthropic API" or "OpenAI API".
- **Through the binary.** The menu guard of the binary only filters input
  while a menu is open, and a press on the dashboard then closes the menu,
  as for every other press there, so no allow-list change was needed. A
  test in `screens/agents.rs` presses By model in the whole window, sends
  what it got through `update_input`, and finds the models table.
- **Tests seen failing first:** `the_models_of_30_days_reach_into_the_month_before`
  and the empty spend test (`otty-agents`, did not compile); the Codex plan
  tests in `rate_limits.rs`, `app_server.rs`, `session_limits.rs` and
  `usage.rs` (old names); the i18n test of "OpenAI API"; in the feature,
  `the_top_section_lists_the_chosen_tab_for_every_profile`, the three
  tests of `model/top_table.rs` (order, limit, "Unknown model", 30 days not
  the month, an empty tab), the four tests of `view/top.rs` (the tabs,
  switching, the models table, the empty tab) and the profile column test,
  all against a stub. The header test and the API key plan tests were
  changed with the removal of `subtitle`, so they failed to compile, not on
  an assertion. The binary test passed at once (the wiring was in place);
  with the reducer not keeping the tab it failed, so it does catch a lost
  press.

### 2026-11-17 (coordinator, owner's follow-up)

The owner saw the table scroll whatever its size. Cause: the section gave
the table the height of the header and its rows, but every row of a
`DataTable` also ends with a one-pixel rule, so the rows were a few pixels
taller than their box and the table scrolled. The kit now has
`data_table::height_for_rows(rows)`, which counts the rules; the top
section's body (table or empty line) always takes the height of five rows,
so it never scrolls and switching tabs moves nothing below. Tests seen
failing first: the kit test that a table given that height does not move
under the wheel, and the view test that the section is as tall for one
row, five rows and none.

