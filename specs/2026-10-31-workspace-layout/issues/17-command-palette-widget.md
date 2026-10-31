# 17 — Command palette widget

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 05, 06, 10
Spec: [../spec.md](../spec.md)

## What

A new widget `widgets/palette/` with the usual files (`event`, `state`,
`reducer`, `model`, `view`, `mod`), registered in `Widgets`. Tests first.

- State: open or closed, query, active index. Opens from the shortcut (ticket 06) and the title bar `TitleSearch` (ticket 11); Esc or a click outside closes it; running an item closes it.
- Items, built when it opens and on query change from other widgets' view models:
  - Sessions: every open tab ("<title>" and the machine), runs `ActivateTab`;
  - Launches: every launch (path in the tree as detail), runs the existing run-launch intent;
  - Actions: New terminal, New launch…, New folder, Settings, Split pane, Close session, Show/hide left panel, Show/hide files, Focus mode — each runs the intent its button already sends.
  No file search (spec Out of Scope).
- Routing: the palette emits an effect naming the chosen item; `app/src/events/palette.rs` turns it into the existing events of the owning widget. The palette never changes other widgets' state itself.
- Keyboard: the input has focus when the palette opens; ↑ ↓ Enter Esc handled by the kit model from ticket 05. While open, terminal panes do not receive keys.
- Search text in the title bar: active session title, or "Search launches and actions" with no tabs.

## Done when

- Reducer tests: open/close, query, run each item kind produces the right effect, close after run.
- Routing tests: each effect maps to the existing event.
- `iced_test` on the view: type, arrow, Enter (spec AC7).
