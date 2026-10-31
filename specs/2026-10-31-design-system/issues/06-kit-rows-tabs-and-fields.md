# 06 — Kit components: rows, tabs and form fields

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## What

The second set of shared components. Same rules as ticket 05.

## Components

| Component | Replaces | Notes |
|---|---|---|
| `TreeRow` | `tree_row_style` in `app/src/style.rs`, row code in both `sidebar_tree.rs` files | Height `row`, indent 18 per level, caret for rows that can open, count on the right in `text-muted`; selected `surface-selected` with `text-strong`, hover `surface-raised` |
| `RailButton` | rail buttons in `app/src/widgets/sidebar/view/mod.rs` | Idle `text-faint`, active `accent` icon plus 2px accent line on the left edge |
| `TabStrip` + `Tab` | `app/src/widgets/tabs/view/tab_bar.rs` styling | Height `tab`, title left-aligned and cut from the end, active tab with `accent` top line; plus right after the last tab |
| `TextField` | `text_input_style` and `text_input_row` | Label above, mono value, hint or error below, optional action button (Browse), `optional` marker |
| `TokenField` | argument list in `wizard_form.rs` (Arguments, Extra arguments) | Enter adds a token, Backspace on empty input removes the last |
| `KeyValueField` | environment rows in `wizard_form.rs` | Narrow name column, wide value column, both mono; cross to remove; Add variable |
| `SettingRow` | rows in `settings_form.rs` | Name and description left, control column 280px right |
| `FormSection` | `section_header` in `wizard_form.rs` | `heading` title, fields `space-4` apart, sections `space-6` apart |

- Extend `examples/gallery.rs` with these components.
- `TokenField` adds and removes tokens: test that logic (Enter with text adds one, Enter with empty input adds nothing, Backspace on empty input removes the last).

## Done when

- The gallery shows all components of this ticket in both themes.

## Comments

- TokenField: `iced` 0.14 text inputs send nothing on Backspace in an empty input. The kit wraps the input in a small private widget (`backspace_guard.rs`, uses the `iced` `advanced` feature, enabled in the kit `Cargo.toml`) that publishes `on_backspace_empty` when the input is focused and empty. Enter uses `on_submit`. The add and remove rules live in the pure `TokenList` type (tested). The input has a fixed 140px width; `iced` cannot grow it to the free space of a wrapped line.
- Text cut: `iced` has no text ellipsis. TreeRow and Tab titles use no-wrap text clipped at the edge, so a long title is cut without `...`.
- TabStrip: tabs that do not fit are clipped at the right edge of the strip together with the plus; there is no scrolling.
- Rows, tabs and fields draw their own caret, cross and plus shapes (`components/glyph.rs`); other icons are still passed in by the caller.
- Token cross uses `IconButton` small (24px), so a token is slightly wider than the 18px cross of the CSS.
- RailButton hover color changes only while the cursor is over the icon itself.
- Gallery arguments `rows`, `fields`, `settings` show one group of the new sections (used for screenshots).
