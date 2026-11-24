# 25 The project page is cards too

Status: ready
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
