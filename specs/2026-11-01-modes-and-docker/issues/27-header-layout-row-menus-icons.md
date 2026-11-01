# 27 — Docker header layout, row ⋯ menus, icon actions in the inspector

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 24, 26
Spec: [../spec.md](../spec.md)

## What

Spec Decision 19, fifth to seventh points. Tests first.

- Header: first row the title and the compose chip; second row the kit
  `Toolbar` with the name filter and icon buttons (Run container / Pull
  image / Create volume / Create network, Prune, Refresh) with tooltips;
  then the table. Narrow widths move buttons into the toolbar's "⋯" menu.
- Every table row with actions shows the kit row "⋯" button; it opens the
  existing row actions menu for that row at the button, without changing
  the selection or the inspector. Right click keeps working.
- Inspector actions become kit icon buttons with tooltips (Remove keeps the
  danger tone); the hint line stays.
- Snapshots (optional, not committed): containers view at 1440 and at a
  narrow width (for example 900 px), Dark and Light, into
  `scratchpad/shots27/`.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: row "⋯" opens the menu for that row and does not change
  the selection.
- `iced_test`: toolbar items emit the same intents as before; at a narrow
  width they are reachable through "⋯"; row "⋯" emits the menu intent;
  inspector icon buttons emit their actions and show tooltips.
