# 09 — Restyle tab strip

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 06
Spec: [../spec.md](../spec.md)

## Details

- `app/src/widgets/tabs/view/tab_bar.rs`: `TAB_BAR_HEIGHT` becomes `TAB` (34), background `surface-panel`, tabs built from kit `TabStrip`.
- The New session plus sits right after the last tab (`IconButton` with `accent` tone), not at the far edge.
- `pane_grid_size` (`app/src/layout.rs`) subtracts the new height; update its tests first.

## Done when

- Spec AC8 for the tab strip; screenshots in both themes.
