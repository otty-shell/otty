# 18 — Divider component and the 1px line token

Status: ready-for-agent
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 17
Spec: [../spec.md](../spec.md)

## What

Every 1px rule in `line` color is drawn by hand today, with its own `1.0`
constant. Move it into the kit.

## Details

- `crates/ui/kit/src/size.rs`: `LINE: f32 = 1.0` — "Width of the 1px `line` rule between panes, panels and under the title bar."
- `crates/ui/kit/src/components/divider.rs`: `horizontal(palette)` and `vertical(palette)` return an element: a container filled with `palette.line()`, `size::LINE` thick and `Length::Fill` long. Add both to the gallery.
- Replace:
  - `app/src/view.rs`: `HEADER_SEPARATOR_HEIGHT` (also used by `layout.rs`) → `size::LINE`; `SIDEBAR_SEPARATOR_WIDTH` and the two hand-made separator containers → `divider::horizontal / vertical`.
  - `app/src/widgets/settings/view/settings_form.rs`: `NAV_SEPARATOR_WIDTH` and its container → `divider::vertical`.
  - `app/src/widgets/terminal_workspace/pane_balance.rs` test constant `PANE_SPACING` → `size::LINE`.
- Pane grid spacing and split lines (`PANE_GRID_SPACING`, `DIVIDER_WIDTH`, `PANE_BORDER_WIDTH`) are ticket 19; leave them.
- No visual change.

## Tests

None for the component (plain styling, AGENTS.md). Existing `layout.rs` tests must still pass.
