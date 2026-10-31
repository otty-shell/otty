# 04 — Failed block look

Status: needs-triage
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03
Spec: [../spec.md](../spec.md)

## Details

- Terminal theme settings get `block_error_fill` and `block_error_edge`; the app fills them from `surface_error()` and `status_error()`.
- A block in state `Failed` is drawn with the error fill under its cells and a 2 px edge on its left side. Selected and hovered fills win over the error fill; the edge stays.

## Tests first

- App: for each theme, the theme foreground on `surface-error` reaches 4.5:1 (spec AC4).

## Done when

- Screenshot of `false` and of a long failing command output in both themes.
