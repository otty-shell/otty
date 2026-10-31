# 04 — Kit: PaneHeader, TitleSearch, ActionTile, FloatingNote, EmptyCard

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Reference: `pane-head`, `tb-search` in `V4Workspace`; `float` note in
`V4Focus`; `empty-card`, `big-action` in `V4Empty`.

| Component | Shape |
|---|---|
| `PaneHeader` | Thin bar above a pane: program name, mono muted folder, gap, optional `Chip`; focused variant on `surface-header`, other panes on `surface-canvas` |
| `TitleSearch` | Title-bar button that looks like a search field: search icon, the focused session title, `Kbd` with the palette shortcut; one press message |
| `ActionTile` | Large row button for the empty card: icon, label, optional `Kbd`; first one can use the accent icon |
| `FloatingNote` | Pill on `surface-overlay` with `shadow-popover`: text parts, muted parts and a `Kbd` |
| `EmptyCard` | Centered column: logo (`LOGO` size, never recolored), `title`, muted description, a stack of `ActionTile`s |

- Gallery sections in both themes.
- `iced_test`: `TitleSearch` and `ActionTile` presses emit their messages.

## Done when

- All five render in the gallery in both themes and match the V4 artboards in size and tokens.

## Answer

Added five components, one file each in `crates/ui/kit/src/components/`, sized from the design CSS (`pane-head`, `tb-search`, `big-action`, `empty-card`, and the floating note in `V4Focus`).

- `pane_header.rs`: `PaneHeader::new(program, &palette).folder(&str).focused(bool).trailing(element)`; 30px, `surface-header` when focused, `surface-canvas` otherwise; `trailing` takes any element (a `Chip` or a state word).
- `title_search.rs`: `TitleSearch::new(title, shortcut, icon, &palette).on_press(msg)`; 480x26 button, search icon, clipped title, `kbd`.
- `action_tile.rs`: `ActionTile::new(icon, label, &palette).shortcut(&str).accent(bool).on_press(msg)`; 44px row, hover `surface-hover`.
- `floating_note.rs`: `FloatingNote::new(&palette).text(&str).muted(&str).kbd(&str)`; parts in call order, 30px pill on `surface-overlay` with `shadow-popover`.
- `empty_card.rs`: `EmptyCard::new(title, description, logo_element, &palette).action(element)`; 460px column. The caller passes the logo as an element (the card never recolors it) and the card gives it a 72x52 slot as in the artboard, not `size::LOGO`.
- New tokens in `size.rs`: `PANE_HEADER` (30), `TITLE_SEARCH_WIDTH` (480), `TITLE_SEARCH_HEIGHT` (26), `ACTION_TILE` (44), `FLOATING_NOTE` (30), `EMPTY_CARD` (460), `EMPTY_LOGO_WIDTH` (72), `EMPTY_LOGO_HEIGHT` (52). Local constants: title search font 12.5 and gap/padding 10; floating note gap 10.

Gallery: sections for all five (empty card uses the repo logo SVG), in both themes through the theme button.

Tests (`iced_test`, written before the code; first run failed to compile): title search sends one message from the title and from the shortcut; action tile sends its message, and nothing without one; empty card shows its parts and only the pressed tile sends; floating note and pane header render their parts.

Not verified: the visual look. The gallery was only compiled, not viewed. Hover colors and the shadow are not covered by tests.
