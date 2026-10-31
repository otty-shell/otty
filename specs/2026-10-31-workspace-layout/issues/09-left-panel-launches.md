# 09 — Left panel: Launches section

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 03, 07
Spec: [../spec.md](../spec.md)

## What

The quick launch tree becomes the lower `PanelSection` "Launches" of the left
panel, as in `V4Workspace`.

- Header actions: New folder and New launch (existing intents `QuickLaunchHeaderCreateFolder`, `QuickLaunchHeaderCreateCommand`). Collapsed header shows the launch count.
- Rows: folders with caret and folder icon; launches with a `TargetDot` (`Ssh` for SSH launches, `Local` otherwise) and the muted word "running" while a launch is active (existing `launching` data).
- Drag and drop, inline rename and the context menu keep working unchanged.
- Section open/closed is view state in the sidebar widget (one bool per section, not persisted).
- English title "Launches"; update all 10 locales.

## Done when

- Quick launch flows of the regression checklist pass, including drag and drop; `iced_test`: header buttons emit the existing intents.
