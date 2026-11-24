# 24 The parts of a page are cards, from one kit component

Status: ready
Blocked by: 23

## Why

Owner, 2026-11-25: the pages draw everything on the plain background, so
it is not clear what belongs to what; on an account page with money the
owner could not tell what belongs to "Needs from you" and what to the
other parts, and not only there. A prototype in OTTY Lite showed three
ways (website repo `src/lite/prototype-blocks/`, page
`/lite/prototype-blocks/`, read only); the owner chose **A, cards**: every
part of a page is a raised card with a 1 px border, rounded corners and
its title inside; "Needs from you" is a card tinted in the warning colour
with a warning border. The card must be a component of the UI kit with
parameters, to be reused everywhere.

## Owner's choices after the prototype (2026-11-25)

- **Needs:** every need row is its heading, its text across the whole
  width of the card, then one line under the text with the plugin label
  (a muted tag) and the button(s), the same in every row and at every
  width; a thin divider between need rows inside the card. This replaces
  issue 21's "label and buttons at the right, under the text only when
  narrow".
- **Data cards (Money, Limits, Credits):** each figure is its own row: the
  label at the left with its own small muted explanation under it, the
  value at the right, a thin divider between rows. "at API prices" is a
  small tag next to the amount, its tooltip with the note and the money
  by kind. Money of an organization has two rows: "Your spend this month"
  (from your Claude Code sessions on this computer) and "Organization
  bill" ("—", shows once OTTY has an admin key). Limits and Credits rows
  keep their bar under the row.
- Prototype: website repo `src/lite/prototype-blocks/` variant A
  (`/lite/prototype-blocks/?variant=a&page=org`).

## What to build

1. **Kit `Card`** (`crates/ui/kit/src/components/card.rs`), in the kit's
   builder style: a title (optional) at the top inside, an optional
   trailing element in the title row (such as the period picker of the
   usage block or a button), the body, and a tone (default; warning, the
   tinted card with the warning border and the dot before the title, as
   the prototype draws "Needs from you"). Raised surface, `size::LINE`
   border, the kit radius, inner padding from the kit spacing, the same in
   dark and light. Check first whether a kit component already does part
   of this (`Surface`, `FormSection`, `PanelSection`, `EmptyCard`,
   `Notice`) and build on it instead of a second one, or say why not.
   Kit tests: the title inside, the body inside the padding, the tone's
   colours.
2. **Agents pages use it:** the account page (Needs from you as a warning
   card, Limits / Credits / Money, Account info), the profile page (Needs
   from you, the usage block with its period picker in the title row,
   Profile info), All accounts and All profiles (the table as a card with
   its title, Needs you as a warning card), the found report if it has
   parts. Every line stays inside the card it belongs to (a need's
   buttons inside the need card, "Your spend this month" and its source
   line inside Money). Space between cards from the kit spacing.
3. **Other places that draw their own card** (grep for hand-made
   bordered containers in feature views: the plugin part cards in
   `plugins/core/src/view/details.rs` and `can_access.rs`, the add
   account dialog, chips if they are cards) use the kit `Card` where it is
   the same thing; leave chips and dialogs that are not cards alone.
4. Pictures before and after (throwaway, not committed) of the account,
   profile and All accounts pages in dark and light; the docs that show
   these pages follow.

## Done when

- Tests first: the kit card lays out its title, trailing element and
  body inside its padding and draws its tone; on the account page the
  need rows and their buttons lie inside the Needs card, the spend lines
  inside the Money card; on the profile page the usage block is one card
  with its period picker in its title row; the plugin page's part cards
  are kit cards.
- The checks of `AGENTS.md` pass.

## Comments
