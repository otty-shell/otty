# 30 — Row press opens the right panel; inspector tabs become a select when narrow

Status: resolved
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 28
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20. Tests first.

- In Docker mode, pressing a table row (a plain left press) opens the right
  panel when it is closed. The row "⋯" button and the right click do not.
  Use the sidebar widget's existing toggle path; no new layer.
- Kit `inspector_tabs`: when the tabs do not fit the given width, render a
  kit `Select` with the same choices instead (built with `responsive`).

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer / events tests: row press with the panel closed opens it; with it
  open nothing changes; "⋯" and right click leave it closed.
- Kit `iced_test`: wide → tabs, narrow → select, both emit the same message.

## Answer

Tests were written first and seen failing (the row press test, and the
narrow-tabs and fit tests), then the code.

- **Row press opens the panel (`events/docker.rs`):** `handle` reduces the
  Docker intent as before. For `DockerIntent::SelectRow` (only a plain left
  press on a row sends it) with the right panel closed, it also runs the
  sidebar's existing `SidebarIntent::ToggleRight` through
  `app.widgets.sidebar.reduce`, so there is no new layer and the focus
  mode rules of that intent still apply. With the panel open nothing
  changes. The row "⋯" button (`RowMenuPressed`) and the right click
  (`OpenRowMenu`) never send `SelectRow`, so they leave it closed.
- **Kit `inspector_tabs`:** built with `responsive`. The width of the tab
  row is measured from the labels (font, padding and the 16 px gaps); if it
  is wider than the space given, a kit `select_owned` with the same
  choices and the same `on_select` replaces it. The signature gained
  `Clone + 'a` on `on_select` and `'a` on the tab type (every caller passes
  a function name).
- **Found on the way:** the tab buttons used a `Fill`-wide underline, so
  the three tabs were spread over the whole width instead of sitting side
  by side (the design is a left-aligned row with a 16 px gap). The
  underline is now as wide as its label. A test pins the positions.
- Gallery: the inspector section has a second tab row in a 120 px frame
  that shows the select.
- No new text, so no i18n change.

Tests: `events::docker` (row press opens a closed panel; with it open it
stays open; "⋯" and right click leave it closed); kit (wide tabs and
their message, narrow select picking the same message, tabs side by side,
tabs at the exact width stay tabs and 8 px less turns into a select).

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.82%
(65614 lines, 8648 missed) on `9c6531e` -> 86.85% (65774 lines, 8648
missed).

## Comments

A row press now reveals the right panel through `DockerEffect::RevealInspector` and `SidebarIntent::RevealRight`: only a press that selected a row does it, it leaves focus mode, and the panel comes back at the width it had before.
