# 11 — Restyle menus and dropdowns

Status: ready-for-agent
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
