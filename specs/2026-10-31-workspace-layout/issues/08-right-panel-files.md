# 08 — Right panel: Files

Status: resolved
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

## Answer

The right panel is now a kit `PanelSection` titled "Files" (always open, not collapsible, takes the full height). Under the header is a muted line: local `TargetDot`, the folder in mono, and "follows <shell>". Below it the tree is unchanged: kit `TreeRow` with `TREE_INDENT`, selected row on `surface-selected`, file press and hover intents as before.

- `app/src/widgets/explorer/view/sidebar_tree.rs`: new `shell_name` prop; the old folder bar is replaced by the section and the folder line.
- `app/src/view.rs`: the right panel passes `app.shell_session.name()` (the same name the tab bar uses for new tabs). No new state.
- `i18n`: new keys `FilesTitle` and `TplFilesFollows` ("follows {shell}") in all 10 locales, plus `i18n::files_follows`.
- The machine dot is `TargetKind::Local` until ticket 10.

Tests (written before the view code): panel shows title, folder and "follows zsh"; clicking a file row sends `NodePressed`; the i18n placeholder test now covers `{shell}`.

Not verified: the visual look and screenshots in both themes (not run). The "shell" is the app's default shell name, not a per-tab program.

Comments: regression-checklist steps to walk: 17 and 18 (Explorer), plus the step that opens and closes the right panel (ticket 07).
