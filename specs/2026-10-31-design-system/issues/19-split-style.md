# 19 — One split style for the sidebar and terminal panes

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 18
Spec: [../spec.md](../spec.md)

## What

`app/src/view.rs` (sidebar split) and `app/src/widgets/terminal_workspace/view/pane_grid.rs`
(terminal splits) each build the same `pane_grid::Style` from `line`, with
different grab zones (8 and 12 px). Move it into the kit with one grab zone.

## Details

- `crates/ui/kit/src/size.rs`: `SPLIT_GRAB: f32 = 8.0` — "Width of the invisible zone around a split line that the pointer can drag." Owner decision: 8 px for both, so the terminal split takes fewer clicks from text near the border.
- `crates/ui/kit/src/components/split.rs`: `style(palette) -> impl Fn(&Theme) -> pane_grid::Style` (hovered region, picked and hovered split all `line`, width `size::LINE`).
- Use it in both places; `.spacing(size::LINE)` and `.on_resize(size::SPLIT_GRAB, …)`. Remove `PANE_GRID_SPACING`, `PANE_GRID_RESIZE_GRAB`, `DIVIDER_WIDTH`, `PANE_RESIZE_GRAB`.
- `PANE_BORDER_WIDTH` (focused pane frame) → `size::LINE`.
- The terminal grid paints its container `line` so the 1px gaps show as rules; keep that, with a short comment.

## Tests

None (styling). Behavior change: terminal split grab zone 12 → 8 px.

## Answer

- Added `size::SPLIT_GRAB` (8 px) and `components::split::style(palette)`, which returns the `PaneGrid` style (all `line`, `size::LINE` wide).
- The sidebar split and the terminal splits both use `split::style`, `.spacing(size::LINE)` and `.on_resize(size::SPLIT_GRAB, …)`. The terminal grab zone went from 12 to 8 px.
- Removed `PANE_GRID_SPACING`, `PANE_GRID_RESIZE_GRAB`, `DIVIDER_WIDTH` and `PANE_RESIZE_GRAB`. The focused pane frame now uses `size::LINE`.
- The terminal grid container is still painted `line`, with a comment saying why.
