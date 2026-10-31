# 08 — Right panel: Files

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 07
Spec: [../spec.md](../spec.md)

## What

The explorer tree in the right panel, as in `V4Workspace` and `V4FileOpen`.

- Panel on `surface-panel`; header "Files" in `PanelSection` style (not collapsible here).
- Under the header a muted line: `TargetDot`, mono folder, "follows <shell>" (the active tab's shell name; data already in the explorer view model or the tabs view model — no new state).
- Rows stay kit `TreeRow`; indent `TREE_INDENT`; selected file `surface-selected`. Open file in the editor works as today.
- New strings go through `i18n` in all 10 locales.

## Done when

- Explorer flows of the regression checklist pass; screenshots in both themes.
