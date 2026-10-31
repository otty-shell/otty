# 07 — Restyle title bar and pane dividers

Status: resolved
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 04, 05
Spec: [../spec.md](../spec.md)

## Details

- `app/src/widgets/chrome/view/action_bar.rs`: height `TITLEBAR` (38), background `surface-chrome`, window controls are kit `IconButton`s, logo-small at 20px high, title in `body` / `text-muted`.
- One neutral 1px `line` under the title bar (`HEADER_SEPARATOR_HEIGHT`, `app/src/view.rs:36`); drop `SEPARATOR_ALPHA` tinting.
- Pane grid spacing and the sidebar separator use `line`, 1px (today 0.5px with alpha).
- Terminal panes sit on `surface-canvas`.
- `screen_size_from_window` in `app/src/layout.rs` keeps working with the new height; its unit tests, if any, are updated first.

## Done when

- Spec AC8 for the title bar; screenshots in both themes.
