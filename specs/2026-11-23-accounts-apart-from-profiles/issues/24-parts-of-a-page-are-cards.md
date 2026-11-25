# 24 The parts of a page are cards, from one kit component

Status: done
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

- 2026-11-25, done. Every part of the Agents pages is a kit card now.

  The kit `Card` (`crates/ui/kit/src/components/card.rs`) is a builder:
  `Card::new(body, palette)` with `.title(..)`, `.line(..)` (a muted
  line under the title, such as "Updated 2 min ago"), `.trailing(..)`
  (an element at the right end of the title row, at its top) and
  `.tone(CardTone::Default | CardTone::Warning)`. `card::rule(tone,
  palette)` draws the thin line between rows inside a card.
  - Default: the surface is `surface-canvas` raised 40 % toward
    `surface-overlay`, so about `surface-header` in OTTY Dark and nearly
    white in OTTY Light, as the prototype draws it. The border is
    `text-muted` at 24 %, `size::LINE` wide, with `radius::RADIUS_LG`.
  - Warning: `status-warn` at 8 % over the page, a `status-warn` border
    at 45 %, a 7 px dot before the title. Its rules are `status-warn`
    at 28 %.
  - Padding 12 px at the top and 16 px at the sides and bottom, 12 px
    between the title row and the body, 16 px between cards on a page.

  Why not build on an existing kit piece: `Surface` fills its parent
  and clips with no border, `FormSection` and `PanelSection` have no
  surface or border and `PanelSection` opens and closes, `EmptyCard` is
  the fixed 460 px start card, and `Notice` is a one-line message with an
  icon and a dismiss cross. None is a bordered part with a title row, so
  the card is a new component. No new palette token: the raised surface
  is a mix of two tokens, as the prototype mixes them.

  Where it is used:
  - Account page: Needs from you (warning), Limits, Credits, Money,
    Account info. A subscription with windows and credits shows two
    cards, Limits and then Credits.
  - Profile page: Needs from you, the usage block (`usage_block::card`,
    the period picker in its title row), Profile info.
  - All accounts and All profiles: the table in a card titled Accounts
    or Profiles, Needs you as a warning card.
  - The Found by OTTY report: the Profiles and Accounts tables as cards
    and the needs as warning cards.
  - Plugins: the part cards of a plugin page (`details.rs`) and of the
    install and update dialogs and the Can access dialog
    (`can_access.rs`) are kit cards in place of their own bordered
    containers.
  - Left alone: the add account dialog's options are radio tiles, the
    chips are chips. The project page and the statistics popups stay as
    they are: this issue does not name them.

  Needs (`view/needs.rs`): every need is its heading, its text across the
  whole width, then one line with the plugin label as a muted kit tag at
  the left and the buttons at the right edge, at every width. Issue 21's
  side-by-side layout is gone. When the line is too short even for the
  tag and the buttons, the label is cut with "…" with a tooltip, as
  before.

  Data cards: each figure is a row, its name and its muted lines at the
  left, its value at the right, the bar under it, thin rules between
  rows. "at API prices" is a kit tag before the amount, with its tooltip,
  and the amount keeps its money by kind on hover. Money of an
  organization has the new row "Organization bill": the bill read with
  the admin key, or "—" with "Shows once OTTY has an admin key"
  (`MoneyViewModel::organization_bill`, `OrganizationBillViewModel`). The
  bill read with an admin key now shows in that row, not as "Spent this
  month". With a bill that failed to read, only the failure line shows,
  as before. Two new words in all ten languages:
  `agents_organization_bill` and `agents_organization_bill_waits`.

  Tests first (`wip: issue 24 tests`), and they failed to build: there
  was no `Card`, `CardTone`, `organization_bill` or new key yet. They
  cover the kit card (title, line, trailing element and body inside the
  padding, no title, the warning dot, both tones' colours and rules in
  both themes), the needs layout at wide, narrow and tiny widths, the
  needs and money rows inside their cards on the account page, Limits
  and Credits as two cards, the usage card with its period in the title
  row, All accounts' cards, the plugin part cards as kit cards, and the
  organization bill in the model. Two older tests changed after the
  code, for the card padding: the value of a money row ends 16 px inside
  the card, and the label of a small button sits 10 px inside the
  button's edge.

  Pictures before and after, dark and light, at 1000 and 600 px:
  throwaway, not committed. One difference from the prototype is left:
  the kit table keeps its canvas background inside a card, so it shows
  as a slightly darker box. Making it transparent would need a new
  `DataTable` option.

  Docs: Agents Overview says what the cards are, the rows of the data
  cards, the two rows of an organization and the new layout of a need.
