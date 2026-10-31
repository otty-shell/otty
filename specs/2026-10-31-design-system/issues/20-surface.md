# 20 — Surface container

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

Five views wrap their content in a container that sets a surface background
and a text color by hand. Move this into the kit.

## Details

- `crates/ui/kit/src/components/surface.rs`: `pub enum Surface { Chrome, Panel, Canvas }` and `surface(content, Surface, palette)` → container with `Length::Fill` size, `clip(true)`, background from the matching `surface_*()` token and `text_color` `text_body()`.
- Use it in: `app/src/components/primitive/sidebar_workspace_panel.rs` (then delete the file, render the hidden state inline in the caller), `app/src/widgets/explorer/view/sidebar_tree.rs`, `app/src/widgets/sidebar/view/mod.rs`, `app/src/widgets/quick_launch/view/sidebar_panel.rs`, `app/src/widgets/chrome/view/action_bar.rs` (bar background).
- Leave containers whose background is conditional (drop target highlight in the quick launch tree) or a pane frame (pane_grid.rs) as they are.
- If a place needs `Length::Shrink` or no clip, give `surface` builder methods only for what is used; no unused options.
- No visual change.

## Answer

- Added `Surface` and `surface(content, Surface, palette)` in `components/surface.rs`. It returns an unfinished `Container` (fill, clip, surface background, body text color), so callers set a fixed size, padding or alignment on it.
- Replaced the containers in the sidebar rail, action bar, explorer directory bar and quick launch panel header. The workspace panel now uses `surface` directly in `view.rs`, with its hidden state rendered inline as an empty `Space`; `sidebar_workspace_panel.rs` is deleted.
- Left the quick launch tree container as is (conditional drop-target background). The explorer's outer wrapper has no background, so only its directory bar changed.
- The fixed-size bars now clip their content, which the old containers did not; their content fits, so nothing changes visually.
