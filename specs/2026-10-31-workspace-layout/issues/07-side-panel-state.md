# 07 — Side panel state: left, right, focus mode

Status: ready-for-agent
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
