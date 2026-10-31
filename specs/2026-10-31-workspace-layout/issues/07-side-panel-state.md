# 07 — Side panel state: left, right, focus mode

Status: resolved
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 01, 06
Spec: [../spec.md](../spec.md)

## What

`SidebarWidget` owns both side panels (spec Decision 8). Tests first.

- Replace `SidebarItem`, `hidden` and `workspace_open` in `widgets/sidebar/state.rs` with: `left_open`, `right_open` (closed by default), `focus_mode`, and a width ratio per side. Keep today's default ratio, collapse threshold and resize-hover suppression; the characterization tests from ticket 01 move over with the same numbers.
- Focus mode hides both panels and remembers what was open; leaving it restores that. Toggling a panel while in focus mode leaves focus mode first.
- Intents: `ToggleLeft`, `ToggleRight`, `ToggleFocusMode`, resize per side. Remove `SelectTerminal`, `SelectExplorer`, `ToggleWorkspace`; `OpenSettings` stays. Wire the three shortcuts from ticket 06.
- Root view (`app/src/view.rs`): left panel | main | right panel, using the kit `split` style and `SPLIT_GRAB`. For now the left panel shows today's quick launch panel and the right panel shows today's explorer tree; tickets 08–10 restyle them.
- Remove the rail view; delete `RailButton` from the kit if nothing else uses it, and its gallery section (ticket 22 updates the design system).
- `SyncTerminalGridSizes` still fires after every change that resizes the main area.

## Done when

- Reducer tests: each toggle, focus mode round trip, toggle during focus mode, resize and collapse per side, grid size sync effect.
- Regression checklist passes; the chrome's `ToggleSidebarVisibility` maps to `ToggleLeft`.

## Answer

### State shape

`app/src/widgets/sidebar/state.rs` (`SidebarState`) now holds:

- `left: SidePanel`, `right: SidePanel`: each is `{ open, ratio }`
  (`app/src/widgets/sidebar/side_panel.rs`). `ratio` is the panel width as a
  share of the window width and is kept while the panel is closed. The left
  panel starts open, the right one closed.
- `focus_mode: bool`.
- `left_split`, `right_split` and `panes: pane_grid::State<SidebarPane>`:
  the root pane grid is `Left | (Main | Right)`. The two split ratios are
  derived from the panel widths after every change (`sync_panes`); the pane
  grid's inner split ratio is relative to the area right of the left panel,
  so a drag on it is converted back to a share of the window.
- unchanged: add menu, cursor, `last_resize_at` (resize hover suppression,
  200 ms).

`SidebarItem`, `hidden`, `workspace_open` and the `Workspace`/`Content`
panes are gone. `SidebarPane` is `Left | Main | Right`.

`SidePanel` exists because both sides follow the same rules (open at least
at the default width, capped; drag; collapse) and change for the same
reason. The rules and numbers are today's: default 0.2, collapse at
`0.2 * 0.2` (0.04), a closed panel dragged past 0.04 opens at 0.04, a small
drag on a closed panel does not resync the grid, reopening keeps the last
width but at least 0.2. The main area always keeps 0.2 of the window, so
one panel's maximum is `0.8 - the other panel's shown width` (0.8 when the
other one is closed, as before).

View model (`SidebarViewModel`): `is_left_open`, `is_right_open` (shown:
open and not in focus mode), `is_focus_mode`, `has_add_menu_open`,
`add_menu_cursor`. The widget also gives `side_panels_ratio()` (both shown
widths together), which `layout::pane_grid_size(screen, side_panels_ratio)`
uses for the terminal grid size; the rail width is no longer subtracted.

### Intents

`ToggleLeft`, `ToggleRight`, `ToggleFocusMode`, `Resized(ResizeEvent)`
(one intent; the split id says which side), `OpenSettings`, and the add
menu and cursor intents as before. Removed: `SelectTerminal`,
`SelectExplorer`, `ToggleWorkspace`, `ToggleVisibility`.

Every toggle emits `SyncTerminalGridSizes`; a resize emits
`QuickLaunchResetInteractionState` and, when a width changed,
`SyncTerminalGridSizes`.

Wiring:

- Shortcuts (`app/src/events/shortcuts.rs`): Left panel → `ToggleLeft`,
  Files panel → `ToggleRight`, Focus mode → `ToggleFocusMode`. The command
  palette still does nothing (ticket 17).
- Chrome: `ChromeEffect::ToggleSidebarVisibility` (logo click) →
  `ToggleLeft`. Interim title bar buttons so the mouse still reaches
  everything the rail had: Files (`ChromeIntent::ToggleFilesPanel` →
  `ToggleRight`) and Settings (`ChromeIntent::OpenSettings` →
  `TabsIntent::OpenSettingsTab`). New i18n key `window_files_panel` in all
  10 catalogs; the Settings tooltip reuses `settings_tab_title`. Ticket 11
  redraws the title bar.

### Focus mode

Focus mode is a flag over the two panels: it does not close them, it only
makes both shown widths zero. Leaving it therefore shows exactly what was
open, at the same widths. A toggle acts on what you see: in focus mode both
panels are hidden, so `ToggleLeft` / `ToggleRight` leaves focus mode (the
other panel comes back as it was) and shows the toggled panel, even if it
was closed before focus mode. Drags are ignored in focus mode.

The view keeps the same pane grid in focus mode (both panels at zero
width) and only drops the drag handles, so the main area keeps its widget
state; before, hiding the sidebar swapped the pane grid for the bare main
area.

### Width choice

Ratios of the window width, for both sides, default 0.2 each. Reasons:
it is what the app does today (the pinned numbers stay valid), it needs no
new pixel/ratio conversion, and the main area minimum (0.2) is one rule for
both sides. On a 1300px window 0.2 is 260px, the design's left `--panel`;
the right panel's 280px (`.panel-right`) is 0.215 there, close enough that
a second default was not worth it. Switching to pixel widths later only
touches `SidePanel` and `sync_panes`.

### Removed

- The rail view (`app/src/widgets/sidebar/view/`), `SidebarItem`, and the
  rail-only icons (`SIDEBAR_TERMINAL`, `SIDEBAR_COLLAPSE`, `SIDEBAR_EXPAND`;
  `SIDEBAR_SETTINGS` / `SIDEBAR_EXPLORER` became `SETTINGS` / `FILES_PANEL`
  for the title bar). The SVG assets stay.
- `RailButton` / `rail_separator` from the kit (`components/rail_button.rs`,
  its `mod.rs` line and its gallery section); nothing else used them.
  `size::RAIL` and `size::RAIL_BUTTON` stay for ticket 22.

### Owner change folded in (Round 2)

The Files panel shortcut on Linux is now Ctrl Shift O
(`app/src/shortcuts.rs`; Ctrl Shift E is IBus emoji input on GNOME). macOS
stays Cmd Shift E. The spec table (Decision 3) and Round Log are updated.
Inside a terminal, Ctrl Shift O no longer sends `^O`; Ctrl O still does.

### Tests (written first, seen failing, then implemented)

| Where | Tests | What they pin |
|---|---|---|
| `widgets/sidebar/reducer.rs` | 25 | default (left 0.2 shown, right hidden); each toggle and its grid sync; focus mode round trip with widths; a panel closed before focus stays closed after; toggle left/right during focus mode; left drag follows, cap 0.8, cap 0.6 next to an open right panel, collapse at 0.03, stays at 0.05, closed panel opens at 0.04, small drag on closed panel does not resync; a left drag keeps the right width (inner split ratio 0.5 / 0.7); right drag follows (0.4), cap 0.6, collapse, closed right opens at 0.04, small drag does not resync; drags ignored in focus mode; `OpenSettings` only opens Settings; add menu; cursor |
| `widgets/sidebar/side_panel.rs` | 9 | one panel's rules: default width, reopen keeps a wide width, reopen at least default, open capped, drag cap, collapse, closed panel drag rules |
| `widgets/sidebar/state.rs` | 5 | pane grid split ratios by default, with the right panel open (0.2 / 0.75) and in focus mode (0 / 1); resize suppression; add menu at cursor |
| `layout.rs` | 4 | pane grid width minus the side panels' share; tab strip height |
| `events/shortcuts.rs` | 3 new | Left panel, Files panel, Focus mode shortcuts send the sidebar intents |
| `events/chrome.rs` | 3 | logo toggle → `ToggleLeft`; Files → `ToggleRight`; Settings → `OpenSettingsTab` |
| `widgets/chrome/reducer.rs` | 2 | the two new title bar intents give their effects |
| `shortcuts.rs` | 1 updated | Linux label table says Ctrl Shift O |

Characterization tests from ticket 01 moved to the per-side API with the
same numbers. Replaced: `SelectExplorer` reopening at 0.2 → `ToggleRight`
opening at 0.2 (and `SidePanel` reopen tests); `SelectTerminal` emitting
nothing → `OpenSettings` emitting only `OpenSettingsTab`; visibility toggle
→ `ToggleLeft`. The old rail-width `layout.rs` tests were replaced by side
panel share tests.

## Comments

- **Checklist walk: not done** (the agent cannot run the GUI). The owner
  should walk all of `regression-checklist.md`, with these steps changed by
  this ticket: 1–2 (start, add menu from the left panel), 8–16 (quick
  launch panel now in the left slot: clicks, context menu, inline rename,
  drag and drop, which depend on the sidebar cursor and resize
  suppression), 17–18 (Explorer is the right panel: open it with the title
  bar Files button or Ctrl Shift O), 19 (Settings: title bar gear or
  Ctrl ,), 21 (new Files tooltip translated), 22 (logo click hides/shows the
  left panel; drag both panel edges wider, narrower and almost closed; the
  two panels together never leave the main area less than 20%; Ctrl Shift F
  hides both and restores them; Ctrl Shift B / Ctrl Shift O in focus mode
  leave it), 23.
- Also check by hand: the right panel's zero-width drag handle sits at the
  window's right edge, next to the window resize grip; dragging it should
  open the panel and not resize the window.
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok (the
  three old `yanked` warnings remain). No new dependencies.
- Coverage (`cargo llvm-cov --workspace --all-features`, lines): base
  `24479ef` 72.50% → this ticket 73.00%.
