# 08 — Restyle widget rail

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 06
Spec: [../spec.md](../spec.md)

## Details

- `SIDEBAR_MENU_WIDTH` (`app/src/widgets/sidebar/constants.rs`) becomes `RAIL` (48).
- Background `surface-chrome`; buttons are kit `RailButton`s; Settings at the bottom.
- Rail icons are tinted in code (`text-faint` idle, `accent` active); never by target.
- Update `pane_grid_size` callers and any test that assumes 52.

## Done when

- Spec AC8 for the rail; screenshots in both themes.
