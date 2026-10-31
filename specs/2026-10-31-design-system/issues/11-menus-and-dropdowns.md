# 11 — Restyle menus and dropdowns

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 05
Spec: [../spec.md](../spec.md)

## Details

Move every overlay menu to kit `Menu`:

- The add menu in `app/src/view.rs` (`ADD_MENU_*` constants; item height becomes `row`).
- Quick launch context menu: `app/src/widgets/quick_launch/view/context_menu.rs`.
- Pane context menu: `app/src/widgets/terminal_workspace/view/pane_context_menu.rs`.
- Menu height math in `app/src/geometry.rs` (`menu_height_for_items`) follows the new item height; update its tests first.
- Items show their shortcut with `Kbd` where a binding exists.

## Done when

- All menus share one look; screenshots of an open context menu in both themes.

## Answer

- Add menu, quick launch context menu and pane context menu use kit Menu; `menu_item.rs` deleted; `menu_height_for_items` follows `ROW` items and kit padding (tests updated first).
- Only Split vertically shows a shortcut (the only real binding), built from the binding's modifiers.
- `menu_panel_style` stays until ticket 13/14 (secret prompt uses it). No destructive tone for Remove/Delete yet.
- Not checked visually: menus open only on click and no input tool is installed. Check them in the final review.
