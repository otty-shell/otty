# 16 — New session split button

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 10
Spec: [../spec.md](../spec.md)

## What

The Sessions header plus becomes a kit `SplitButton`, as in `V4Palette`:

- Plus → New terminal.
- Chevron → kit `Menu`: New terminal (shortcut), Split current pane right (shortcut; disabled without a terminal tab), separator, label "Launches" and each launch at the top level of the tree (runs it), separator, New launch….
- This replaces the old add menu (`view_add_menu_overlay` in `view.rs` and the `AddMenu*` sidebar intents): New folder is in the Launches header (ticket 09). Remove what is left unused.

## Done when

- Every menu item runs an existing intent (`iced_test`); the add menu code is gone; regression checklist passes.
