# 14 — Remove old styling code

Status: resolved
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 07, 08, 09, 10, 11, 12, 13
Spec: [../spec.md](../spec.md)

## Details

- Delete what the kit replaced: `app/src/style.rs` (keep `thin_scroll_style` only if no kit equivalent; then move it to the kit), `app/src/components/primitive/icon_button.rs` and `menu_item.rs`, local style functions in widget views.
- `StyleOverrides` in `app/src/theme.rs`: remove if nothing uses it after the migration.
- Check AC7 and AC8 with `grep`; list any local size constants that stay, each with a doc comment.
- Run the full check list from AGENTS.md and compare coverage with the merge base (AC12).

## Done when

- AC7, AC8, AC12, AC13 hold.

## Answer

- Deleted `app/src/style.rs`, `components/primitive/icon_button.rs`, the never-called `tabs/view/tab_content.rs` (and its two i18n keys), `IcedColorPalette`, `StyleOverrides`, the `BUTTON_*` layout constants. The iced `Theme` is built from `UiPalette`.
- New kit `scrollbar` replaces `thin_scroll_style`; pane borders use `focus` / `line`; the exit banner and secret prompt backdrop use tokens.
- AC7: 0 hits for `iced_palette|IcedColorPalette|extended_palette|Color::from_rgb` in app widgets and components. AC8: 20 local `f32` constants remain, each documented (widths, 1px dividers, resize grips, logo size, focus trap, ratios).
- AC13: line coverage 69.81% (33035 lines, 9972 missed) vs 69.53% baseline on cf3c389; 26 business-logic tests added (quick launch drag and drop and context menu, sidebar state).
- Small color changes: scrollbar, unfocused pane border, exit banner, lighter OTTY Light modal backdrop, quick launch whole-tree drop highlight on `surface-selected`. Not checked visually.
