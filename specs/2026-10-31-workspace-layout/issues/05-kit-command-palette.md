# 05 — Kit: CommandPalette

Status: resolved
Type: task
Class: A + tested model
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

The palette's look and its list logic, without knowing any app data.
Reference: `ot-palette` in `V4Palette`, design system card `CommandPalette`.

- View: `radius-lg`, `surface-overlay`, `shadow-popover`; an input row (search icon, text input, `Kbd` Esc); group labels; items with optional `TargetDot` or icon, title with the matched part in `text-strong` semibold, optional muted detail and `Kbd`; the active item on `surface-hover`; a footer with key hints. Width from a new `size::PALETTE` token.
- Model (plain Rust, in the kit because it has no app meaning): items with a group, title and detail; `filter(query)` does case-insensitive substring match on title and detail and returns the match ranges; group order is kept; active index moves with up/down, wraps, and resets to the first item when the query changes; empty groups are hidden.
- Messages: query changed, move up, move down, run active, run item by index, close.

## Done when

- Table tests for the model: empty query shows everything, match ranges, wrap at both ends, reset on new query, no items.
- `iced_test`: typing changes the query; clicking an item emits "run item"; gallery shows a filled and an empty-result palette in both themes.

## Answer

Added the palette in two files in `crates/ui/kit/src/components/`, sized
from the design CSS (`ot-palette`, `ot-palette-input`, `ot-palette-item`,
`ot-menu-label`, `ot-palette-foot`) and the V4Palette structure.

### Model: `palette_list.rs`

Plain Rust, no app meaning. `K` (the caller key) is the only generic.

- `PaletteItem::new(key, group, title).with_detail(&str).with_target(TargetKind).with_shortcut(&str)`;
  getters `key()`, `group()`, `title()`, `detail()`, `target()`, `shortcut()`.
- `PaletteList::new(items)`: empty query, everything matches, first entry active.
- `set_query(String)`, `move_up()`, `move_down()`.
- Reads: `query()`, `active() -> Option<usize>` (index among the matches),
  `match_count()`, `matches()` (each item with the byte ranges of its title
  that match), `active_key() -> Option<&K>`, `key_at(index) -> Option<&K>`.

Rules: matching ignores case and looks for the trimmed query inside the
title or the detail (a query of only spaces matches everything). Title
ranges are all non-overlapping places of the query, as byte ranges of the
original title; lower-casing is done char by char with a map back to the
original chars, so ranges stay on char boundaries even when lower-casing
changes the byte length (tested with Cyrillic, Japanese and `İ`). A match on
the detail only shows the item with no highlight. Groups keep the order in
which they first appear in **all** items (so they do not jump while
typing); groups with no match are left out. The active index wraps at both
ends, goes back to 0 on every `set_query`, and is `None` when nothing
matches.

### View: `command_palette.rs`

```rust
CommandPalette::new(&list, search_icon, &palette)
    .placeholder("Search sessions, launches and actions")
    .no_matches("No matches")
    .hint("↑↓", "move")
    .hint("Enter", "open")
    .on_action(Message::Palette)
```

It emits one enum, `PaletteAction`, mapped by `on_action`:
`QueryChanged(String)`, `MoveUp`, `MoveDown`, `RunActive`, `RunItem(usize)`
(index among the matches, as `key_at` takes it) and `Close`. Without
`on_action` the palette is drawn but inert. All text comes from the caller
(placeholder, no-match line, footer hints), except the key name `Esc`. The
search icon is passed in, like `TitleSearch`.

Look: 560 wide, padding 6, `surface-overlay`, `radius-lg`, `shadow-popover`;
input row 40 tall (14px icon, input in UI 14 `text-strong`, `Kbd` Esc),
`line` under it, 4px gap; group labels reuse the menu heading
(`.ot-menu-label`); entries 32 tall with optional target dot, title (matched
pieces in `text-strong` semibold, rest in the item text color), muted mono
detail (11.5) and `Kbd`; the active entry and the hovered one use the menu
item style (`surface-hover`, `text-strong`); footer gap `space-5`, padding
8 10 4, caption 11.5 muted. The list scrolls after 12 entries (local
constant; the design gives no limit).

### Keyboard: the kit view handles the keys itself

While the palette input has focus, ↑ sends `MoveUp`, ↓ sends `MoveDown`,
Esc sends `Close` (instead of the default `text_input` behavior of just
dropping focus), and Enter sends `RunActive` (the input's `on_submit`). The
keys are caught by a small private wrapper around the input. The old
`backspace_guard.rs` (used by `TokenField`) became that general wrapper,
`input_keys.rs` (`InputKeys::new(input).on_key(Named, message)`), so both
components share it; `TokenField` keeps its behavior (new `iced_test`
characterization tests written before the change). When the input does not
have focus, the palette sends nothing for keys.

For ticket 17 this means: the app does **not** route ↑ ↓ Enter Esc to the
palette. It only has to (1) focus the input when it opens the palette and
(2) map each `PaletteAction` to `PaletteList` calls or to running the item.
If focus can leave the input while the palette is open (for example a click
on the palette background), keys stop working until the input is focused
again; the app can refocus it on any palette action, or close the palette on
a click outside.

### Focusing the input

`command_palette::input_id()` returns the fixed id
`otty-command-palette-input`. On open:

```rust
iced::widget::operation::focus(command_palette::input_id())
```

The id is fixed, so only one palette should be on screen at a time.

### New tokens and shared helpers

- `size::PALETTE` (560), `size::PALETTE_INPUT` (40), `size::PALETTE_ITEM` (32).
- Local constants: input gap 10, side padding 10, icon 14, input font 14,
  list gap 4, footer padding, list max height (12 entries).
- `menu::item_style` and `menu::heading_view` are now `pub(super)` so the
  palette entries and group labels share the menu look.

### Gallery

Section "CommandPalette": the V4Palette entries with query `la` (Launches,
Sessions, Actions; first entry active, `la` in bold; ↑ ↓ Enter and typing
work) and the same list with query `docker` showing the "No matches" line.
Both follow the theme button (OTTY Dark and OTTY Light).

### Tests (written before the code; first runs failed on `todo!()`)

- `palette_list.rs` (9): empty query shows all by group; spaces-only query;
  a 14-row match table (case, several matches, detail-only match, no match,
  Cyrillic, Japanese, `İ`, char-boundary check); group order while filtered;
  wrap table for up/down; reset to the first match on a new query; no match;
  no items; keys follow display order.
- `command_palette.rs` (9, `iced_test` + one table test): typing sends
  `QueryChanged`; clicking an entry sends `RunItem` with its display index;
  ↓ ↑ Enter Esc in the focused input each send their action; keys without
  focus send nothing; clicking the Esc hint closes; groups, detail, shortcut
  and hints are shown; a query splits the title at the match and hides empty
  groups; the no-match line; `title_pieces` table (with Cyrillic).
- `token_field.rs` (2, characterization for the wrapper change): Backspace
  in the empty focused input is reported; with text it edits the text.

### Not verified / open

- The visual look: the gallery was only compiled, not viewed.
- Moving the active entry with ↑ ↓ past the visible part of a long list
  does not scroll it into view. With today's item count (sessions, launches,
  about ten actions) the list rarely scrolls; if ticket 17 needs it, the
  list can get a scrollable id and the app can snap to the active entry.
- Coverage (`cargo llvm-cov --workspace --all-features`, line): previous
  commit `24479ef` 72.51% → this ticket 73.76%.
