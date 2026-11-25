# 25 The project page is cards too

Status: done
Blocked by: 24

## Why

Owner, 2026-11-25: after issue 24 the account, profile and All pages are
cards; the project page should be cards too.

## What to build

1. The Agents project page (`crates/app/features/agents/core/src/view/project.rs`
   and its model) draws its parts with the kit `Card` of issue 24: the
   project info (the facts and the profile chips) as a card titled
   "Project info", and the sessions table as a card titled "Sessions"
   whose title row carries what today sits over the table (the search or
   filter, New session and the like, if any) as the card's trailing
   element. Needs, if the page shows any, as a warning card.
2. The page stays a table screen (issues 38/39 of spec
   2026-11-19-agent-plugins): it keeps the screens' minimum width, scrolls
   sideways below it, and the sessions table still scrolls down on its own
   under its sticky header, now inside its card, filling the height left;
   nothing scrolls twice, and the table still fits the smallest screen
   inside the card's padding (check the column minimums).
3. The statistics popups of a project and of a session use cards where
   they have parts, as the pages do.
4. Pictures before and after at a wide and a narrow width, dark and light
   (throwaway, not committed); docs follow.

## Done when

- Tests first: the project info and the sessions table each lie inside
  their card; the sessions table fills the card's height and scrolls down
  inside it; the page keeps the minimum width and the table fits the
  smallest screen inside the card.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-25, done. The project page and both statistics popups are kit
  cards now.

  Project page (`view/project.rs`): the title row stays as it was, then
  the notice of a gone folder when there is one, then two cards 16 px
  apart, 20 px under the title row. "Project info" holds the path, the
  profile chips and the facts. "Sessions" has the search and Refresh as
  its trailing element, 360 px wide (the whole field, the gap and the
  button), and its body is the tags, the table and the line that says how
  to continue a session. The body fills the height, so the card takes
  what the title row and Project info leave, and the table fills the card
  and scrolls down on its own under its sticky header. The page is still
  a `MainContent::TableScreen`: it keeps `size::SCREEN_MIN`, scrolls
  sideways below it, and nothing scrolls twice. The page shows no needs,
  so there is no warning card.

  Column minimums: inside the card the table has 32 px less, 528 px at
  the smallest screen. The four minimums and the "⋯" column took 560 px,
  so the minimum of the Title column went from 198 to 166 px, as in
  issue 38: it has the widest share and ends with "…".

  Popups (`view/stats_dialog.rs`): a session shows "Session info" (its
  facts), "Limits" (its usage windows, when its agent writes them) and
  the usage card; a project shows "Project info" and the usage card. The
  usage card is the same `usage_block::card` as on the profile page, the
  total under its title and the period at the right end of its title
  row, so the old `usage_block::view` and `blocks::title` and
  `blocks::titled` are gone. One new word in all ten languages:
  `agents_session_info` ("Session info").

  Tests first (`wip: issue 25 tests`): the new key did not exist yet, so
  the crate failed to build. With the key stubbed for one run, the eight
  new and changed tests failed: Project info and the sessions table
  inside their cards, the search in the title row, the table filling the
  card with 2 and with 40 rows, the table fitting the smallest screen
  inside the card, Refresh at the right end of the card, and the parts of
  both popups as cards. One older usage block test changed after the
  code: the mark "at API prices" ends 16 px inside the card.

  Pictures before and after, dark and light, at 1000, 592 and 480 px for
  the page and at 1000 and 600 px for the popups: throwaway, not
  committed. Left as it is: in OTTY Dark a field (the search, the period)
  has about the colour of the card under it, so its box is faint. The
  usage card of the profile page has had the same since issue 24. Making
  it stand out would be a kit change of the field or the card colours.

  Docs: Agents Overview says the project page and the statistics are
  cards.
