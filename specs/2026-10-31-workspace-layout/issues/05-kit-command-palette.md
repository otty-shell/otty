# 05 — Kit: CommandPalette

Status: ready-for-agent
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
