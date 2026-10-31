# 10 — Restyle sidebar trees

Status: ready-for-agent
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
