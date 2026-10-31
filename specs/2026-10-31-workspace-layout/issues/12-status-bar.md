# 12 — Status bar

Status: ready-for-agent
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 02, 10
Spec: [../spec.md](../spec.md)

## What

Kit `StatusBar` at the bottom of the window, as in `V4Workspace` (spec
Decision 9: a view in `widgets/chrome/view/status_bar.rs`, no new widget).

- Left: machine of the active session (dot + name), shell name, mono folder of the focused pane. Right: "N panes" for the active terminal tab, "N sessions".
- With no tabs (empty state): machine on the left, "OTTY <version>" on the right.
- In focus mode the status bar stays.
- A small `StatusBarViewModel` built from the tabs, terminal workspace and explorer view models; pure function.

## Done when

- View model tests: terminal tab, settings tab, no tabs, folder change, pane count after split and close (spec AC8).
