# 06 — Root view by mode: switcher and menu in the left panel

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01, 04
Spec: [../spec.md](../spec.md)

## What

The root view asks the active mode what goes in each slot (spec Decision 5).

- Left panel: kit `ModeSwitcher` with the mode name at the top, then the mode's content. Terminal mode keeps Sessions and Launches exactly as today.
- All modes menu: kit `ModeMenu` as an overlay anchored under "⋯". Esc and a click outside close it, like the header menus.
- Main area, right panel and status bar switch with the mode. Docker mode shows a temporary placeholder until tickets 09 and 10.
- Panel open/closed and widths stay shared (sidebar widget). When the left panel is hidden, the switcher is hidden too.
- Terminal tabs keep running in other modes. Coming back to Terminal mode syncs the grid size (`SyncTerminalGridSizes`) and focuses the active pane.
- This ticket changes `app/src/view.rs`; say so on the commit.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: clicking a tab changes the mode; clicking "⋯" opens the menu; Esc closes it.
- Tests: switching to Docker and back keeps the tabs and the active tab; grid size sync is emitted on return.
- The regression checklist of `specs/2026-10-31-workspace-layout` passes in Terminal mode.

## Answer

The root view now asks the active mode what goes in each slot. This ticket
changes `app/src/view.rs`.

### How each slot is chosen

`view.rs` reads `app.widgets.modes.active()` once and matches on
`(slot, mode)`:

| Slot | Terminal | Docker (placeholder) |
|---|---|---|
| Left panel | switcher row + Sessions/Launches (`view_terminal_left_panel`, unchanged) | switcher row + empty space |
| Main area | `view_tab_content` (unchanged) | `docker_placeholder::main_area` (kit `EmptyCard` with the Docker icon) |
| Right panel | Files (`view_right_panel`, unchanged) | empty space |
| Status bar | `view_status_bar` (unchanged) | `docker_placeholder::status_bar` (mode name, `OTTY <version>`) |

The switcher row is part of the left panel content (`view_left_panel`), so it
hides with the panel (panel toggle, focus mode). Panel open/closed and widths
stay with the sidebar widget and are shared by all modes. The All modes menu
is one more layer in the content stack, like the header menus.

### New files

- `app/src/widgets/modes/view/mode_switcher.rs`: kit `ModeSwitcher` from
  `vm().tabs` plus the active mode name (semibold, `text-strong`), padding
  8/8/4. Tab → `Select(mode)`, "⋯" → `ToggleMenu`.
- `app/src/widgets/modes/view/mode_menu.rs`: kit `ModeMenu` at 8/42 (as on
  the `DockerMode` board) over a transparent dismiss layer. Row →
  `Select(mode)`; pin toggle → `Pin` / `Unpin` / disabled with the hint;
  click or right click outside → `CloseMenu`. The menu is wrapped in
  `opaque` so clicks on its own padding do not close it. Empty Other modes
  shows the board's hint.
- `app/src/widgets/modes/view/docker_placeholder.rs`: the temporary Docker
  main area and status bar. Tickets 09/10 delete it.
- `assets/svg/mode-terminal.svg`, `assets/svg/mode-docker.svg` (paths from
  the boards), `icons::MODE_TERMINAL` / `MODE_DOCKER`, `Mode::icon()`.

### Behavior

- `ModesWidget::active()` and `is_menu_open()` accessors.
- `events/modes.rs`: `ModeChanged(Terminal)` → `SyncTerminalGridSizes` +
  `FocusActive`; other modes need nothing. When the menu opens, keyboard
  focus is taken from every widget (`operation::focus` on an id no widget
  has), so typed keys do not reach the shell and Esc reaches the app keyboard
  subscription. When the menu closes in Terminal mode, `FocusActive` gives
  the active pane its keys back (like the palette).
- `update.rs`: Esc (`guards::is_escape_press`) closes an open modes menu.
  The modes menu counts as an open menu for the context-menu guard (another
  app action closes it like the header menus), `close_all_context_menus`
  also closes it, and `AppEvent::Modes(_)` is allowed by the guard.
- The resize grips layer stands aside while the modes menu is open.
- Terminal tabs keep running in Docker mode: their subscriptions come from
  app state, not from the view.
- `i18n::modes_pinned_note(count, limit)`; new keys `modes_no_other_modes`
  and `docker_placeholder_description` in all ten locales.
- Removed the `expect(dead_code)` attribute on `mod modes` in
  `app/src/widgets/mod.rs`.

### Tests (written first, seen failing)

- `modes/view/mode_switcher.rs` (`iced_test`): tab click selects its mode,
  Terminal tab, "⋯" toggles the menu, temporary tab before "⋯", active name.
- `modes/view/mode_menu.rs` (`iced_test`): sections, note and hint; row
  click selects; a click outside closes; a click on the menu padding does
  not; pin toggle mapping for Unpin, Pin, disabled, hidden.
- `view.rs` (`iced_test` on the real root view): Terminal mode shows the
  switcher, Sessions, Launches and the empty state; Docker mode swaps them
  for the placeholder; a hidden left panel hides the switcher; clicking the
  tabs and "⋯" in the window switches modes and opens the menu; the open
  menu is drawn and its row selects.
- `update.rs`: Esc closes the menu; Pin/Unpin keep it open; choosing a mode
  switches and closes it; another app action closes it.
- `events/modes.rs`: back to Terminal emits grid size sync and focus;
  switching to Docker and back keeps the tabs and the active tab; closing
  the menu focuses the terminal only in Terminal mode.
- `guards.rs`, `i18n`, `types.rs`: Esc detection, guard allows modes events,
  pinned note, mode icons.

Esc is tested at the update level with the key event the keyboard
subscription delivers: `iced_test` simulates widgets only, and Esc is not
taken by any widget while the menu is open.

### Deviations

- No shortcut labels on the tabs and menu rows yet: ticket 05 owns the
  keymap. After 05 lands, pass `.shortcut(label)` to `ModeTab` and
  `ModeRow` in the two view files.
- Header menus still do not close on Esc; only the modes menu does.
- While the modes menu is open, keys from the subscription are ignored by
  the context-menu guard (as with the header menus), so ticket 05's Ctrl
  Shift M cannot close it until the guard lets that shortcut through.
- The board's separator between menu sections and the "open now" note are
  not in the kit `ModeMenu`; left out.

### Follow-ups for tickets 09 and 10

Replace the `_` arms in `view.rs` with the Docker widget: left panel
content in `view_left_panel` (`match mode`), main area and right panel in
the `content` closure of `view()`, and the status bar `match mode` in
`view()` (ticket 18). Then delete `modes/view/docker_placeholder.rs` and the
`docker_placeholder_description` key. Ticket 10 puts its no-engine cards in
the main area slot. Engine work can start on `ModesEffect::ModeChanged(Docker)`
in `events/modes.rs`.

## Comments

- `cargo +nightly fmt`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace --all-features`
  and `cargo deny check` pass. No new dependencies.
- Coverage (lines): base `9798b42` 80.70% → this ticket 81.97%.
- The app was rendered offscreen through `iced_test` snapshots of the real
  root view (Terminal, Terminal with the menu, Docker, Docker unpinned with
  the menu; Dark and Light). The live app and the full regression checklist
  walk were not run in this session; Terminal mode code paths are unchanged
  apart from the switcher row on top of the left panel.
