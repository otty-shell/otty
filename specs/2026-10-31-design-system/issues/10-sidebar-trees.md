# 10 — Restyle sidebar trees

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 06
Spec: [../spec.md](../spec.md)

## Details

- `app/src/widgets/explorer/view/sidebar_tree.rs` and `app/src/widgets/quick_launch/view/sidebar_tree.rs` render rows with kit `TreeRow`; their local `TREE_*` constants go away.
- Panel background `surface-panel`; panel header (`quick_launch/view/sidebar_panel.rs`) uses `heading` text and kit `IconButton`.
- Selected row `surface-selected` (no more `dim_blue` with alpha 0.7).
- Drag and drop targets in quick launch keep working; check `DropTarget` highlighting uses `accent`.
- If `crates/ui/tree` hard-codes row height or indent, pass them in from the app instead of changing defaults for other users of the crate.

## Done when

- Spec AC8 for rows; screenshots of both trees in both themes.

## Answer

- Both trees use kit TreeRow (gained `hovered`, `drop_target`, `icon`); panel headers are 34px (`size::TAB`) so they line up with the tab strip; English panel title "SHELL" became "Quick launch", matching the other 9 locales.
- `crates/ui/tree` unchanged. `tree_row_style` in `app/src/style.rs` stays until ticket 12 (settings navigation still uses it).
- Not checked visually: the Explorer tree (no input tool to open it during the run). Check it in the final review.
