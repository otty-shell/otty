# 18 — Launch editor and launch context menu

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

`widgets/quick_launch/view/wizard_form.rs` and `context_menu.rs` in the
`V4Launch` look. Same fields, checks and buttons as today (spec Out of Scope
lists what is not added: Browse, live checks, Ctrl S, unsaved footer).

- Form: `FormPage` with title "Edit launch" / "New launch" and the muted line "Saved launches open a new session with one click."; sections Title, Type (`Segmented`), Connection or Program, Arguments (`TokenField`), Environment (`KeyValueField`), Working directory, After it exits (`Segmented`) with its muted hint; field labels mark optional fields with a muted "optional". Existing validation errors show under their field in `small` `status-error` text.
- Actions at the bottom right: Cancel, then the primary Save.
- Context menu: same actions as today (`ContextMenuAction`), ordered as in `V4Launch` with separators; Delete uses the danger style; shortcut labels only for shortcuts that exist.

## Done when

- Quick launch editor flows of the regression checklist pass; existing reducer tests unchanged; screenshots in both themes.
