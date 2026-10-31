# 06 — Kit components: rows, tabs and form fields

Status: ready-for-agent
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
