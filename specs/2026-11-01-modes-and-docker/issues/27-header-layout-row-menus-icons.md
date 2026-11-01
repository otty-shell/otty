# 27 — Docker header layout, row ⋯ menus, icon actions in the inspector

Status: resolved
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

## Answer

The Docker header is two rows with the kit `Toolbar`, every row with
actions ends with a "⋯" button, and the inspector actions are icon buttons.
Tests were written first and seen failing (12 of the new tests), then the
code.

### Header (`view/main_area.rs`)

- Row 1 (`title_row`): the title and, with a project filter on, the
  "compose: <name>" chip with its clear button.
- Row 2 (`toolbar`): kit `Toolbar` with the name filter as its field (140
  to 240 px, shrinks first) and three `ToolbarItem`s: the create button
  (Run container / Create volume / Create network with the plus icon, Pull
  image with the pull icon; disabled while `can_create` is off), Prune
  (prune icon, its label "Prune stopped" / "Prune dangling" / "Prune
  unused", disabled while a prune runs) and Refresh. The labels are the
  existing texts and serve as tooltips and, once the buttons move into "⋯",
  as menu item texts. The intents are the same as before (`OpenCreate`,
  `Prune`, `Refresh`, `FilterChanged`).
- The table gets `.on_row_menu(i18n "More actions", …)` sending the new
  `DockerIntent::RowMenuPressed { index, position }`; rows whose item has
  no actions (a container being removed) use `.row_without_menu`
  (`TableViewModel::rows_without_actions`, filled in `model::table`).
  Right press still sends `OpenRowMenu`.

### Row menu without selection (reducer)

- `DockerState` keeps the open row menu as `RowMenu { position, target }`
  (target: the row's id in the open list). New getters `row_menu_target`
  and `selected_id`; `shown_row_id(index)` was split out of
  `select_shown_row` (same sorted, filtered order).
- `action_flow`: `select_and_open_row_menu` (right press: select, then
  open) and `open_row_menu` (row "⋯": open only; the selection and so the
  inspector stay). `act` runs on the menu's row when a row menu is open,
  otherwise on the selected row; `selected_item` became
  `target_item(state, id)` (looked up with the new `ResourceList::find`).
- `action_model::row_menu` lists the actions of the menu's row
  (`row_actions(state, id)`, the old body of `action_buttons`, which now
  passes the selected id). Disabled actions stay in the menu, disabled,
  with the same rules and hints as before.

### Inspector (`view/inspector.rs`)

Each action is an `IconButton` (medium) with its label as tooltip,
`IconTone::Danger` for Remove and `on_press_maybe` for the enabled state;
the hint line under the buttons stays. Icons: Shell → `shell`, Start and
Run → `start`, Stop → `stop`, Restart → `restart`, Pause → `pause`,
Unpause → `resume`, Remove → `trash`, Pull again → `pull`
(`crate::icons` gained `PLUS`, `PULL`, `PRUNE`, `START`, `STOP`,
`RESTART`, `PAUSE`, `RESUME`, `TRASH`, `SHELL`).

### i18n

One new key, `DockerMoreActions` ("More actions"), the tooltip of the
toolbar "⋯" and the row "⋯", in the ten locales; `Key::ALL` is 401. No key
became unused (the old button labels are now tooltips).

### Tests

Reducer: the row "⋯" opens that row's menu at the position and keeps the
selection; a menu item runs on that row, not the selected one; Remove from
it asks about that row; after the menu closes the inspector actions act
on the selection again; past the last row nothing opens. View model: the
row "⋯" menu lists its row's actions while the inspector keeps the
selected row's. `iced_test` (main area): title and chip above the filter
row; no button labels as text; create (disabled and enabled), Prune
(enabled and disabled) and Refresh send their intents; at 250 px Prune and
Refresh are reached through "⋯" and the create button and the filter still
work; the row "⋯" sends `RowMenuPressed` for that row; a row without
actions has no "⋯" (a press there selects the row). Inspector: icon
buttons send `Act`, no labels as text, a disabled button sends nothing and
the hint shows, the hovered button shows its label (snapshot hash, as in
the kit).

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.56%
(64453 lines, 8664 missed) on `e0faab4` → 86.64% (64846 lines, 8664
missed).

### Deviations and choices

- The row "⋯" has its own intent (`RowMenuPressed`) rather than reusing
  `OpenRowMenu`, because a right press must keep selecting the row.
- The row menu opens at the bottom-left corner of the "⋯" button through
  the existing `anchor_position`, so near the right edge it is moved left
  to stay inside the window.
- Start and Run share the play icon (they never show on the same list).
- Snapshots were taken with a throwaway root-view test (not committed):
  containers at 1440 × 900 and 900 × 700, plus the row "⋯" menu at 1440, in
  Dark and Light. At a 900 px window the main area is still about 600 px
  wide, so all toolbar buttons fit; the collapsing is covered by the
  250 px view tests and the kit gallery. The inspector at 900 px is narrow
  (the panel widths come from before this ticket) and its buttons wrap.
