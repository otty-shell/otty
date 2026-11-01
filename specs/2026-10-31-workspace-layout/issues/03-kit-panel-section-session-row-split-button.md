# 03 — Kit: PanelSection, SessionRow, SessionGroup, SplitButton

Status: resolved
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 01
Spec: [../spec.md](../spec.md)

## What

Building blocks of the left panel. Reference: `sec`, `sec-head`, `ot-group`,
`ot-session` in `V4Workspace` and `V4Palette`, and the design system card
`SessionRow`.

| Component | Shape |
|---|---|
| `PanelSection` | Header: caret (open/closed), `heading` title, optional count or note, up to three `IconButton` (`sm`) actions on the right; body below when open; one section can be marked to take the remaining height. Emits a toggle message |
| `SessionGroup` | Group label row: `TargetDot`, machine name, muted kind ("this computer", "ssh") |
| `SessionRow` | `row` (30) tall, indented under its group; title (ellipsis is not available in iced 0.14, keep clipping as today), optional muted state word ("connecting", "editor"), selected = `surface-selected`, hover `surface-raised`; press, right press and close messages |
| `SplitButton` | Primary `IconButton` plus a narrow chevron button sharing one hover frame; two messages |

- Gallery sections in both themes, including a `PanelSection` holding `SessionGroup` + `SessionRow`s and a collapsed one.
- `iced_test`: section toggle, row press, split button parts each emit their own message.

## Done when

- Every state (idle, hover, selected, collapsed) is visible in the gallery in both themes.

## Answer

Added four components, one file each in `crates/ui/kit/src/components/`, sized from the design CSS (`sec`, `ot-group`, `ot-session`, `ot-state*`, and the split button in V4Palette).

- `panel_section.rs`: `PanelSection::new(title, &palette).open(bool).grow(bool).count(usize).note(&str).action(element).body(element).on_toggle(msg)`; at most three actions, body shown only while open, `grow` takes the remaining height and clips.
- `session_group.rs`: `SessionGroup::new(name, TargetKind, &palette).note(&str)`; 28px tall, dot, name, muted kind.
- `session_row.rs`: `SessionRow::new(title, &palette).selected(bool).state(word, SessionTone).on_press(msg).on_right_press(msg).on_close(msg)`; `SessionTone::{Muted, Waiting, Done, Failed}` with `color(&palette)`; 30px tall, title clipped, close cross shown when `on_close` is set.
- `split_button.rs`: `SplitButton::new(icon, label, menu_label, &palette).menu_open(bool).on_press(msg).on_menu(msg)`; small accent `IconButton` plus an 18px chevron in one `radius-sm` frame that shows `surface-raised` while the menu is open.
- New tokens in `size.rs`: `GROUP` (28), `SESSION_INDENT` (23, computed from `space-3`, dot and `space-3`), `SPLIT_CHEVRON` (18). `icon_button::tooltip_hint` is now `pub(super)` so the chevron shares the tooltip.

Gallery: a `PanelSection` with groups and rows (selected, state words, long title), a collapsed section, and split buttons idle and with the menu open, in both themes through the theme button.

Tests (`iced_test`): section title sends toggle; header action sends only its own message; body shows only while open; extra actions are dropped; row press, right press and close each send their own message; tone colors map to tokens; split button primary and chevron each send their own message.

Not verified: the visual look. The gallery was only compiled, not viewed. Hover frames and the open-menu frame are not covered by tests. The font size 12.5 in `SessionGroup` is a documented local constant, as in the other kit components.
