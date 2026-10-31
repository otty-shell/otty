# 18 — Launch editor and launch context menu

Status: resolved
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

## Answer

The launch editor and its context menu now follow `V4Launch`.

- `wizard_form.rs`: page heading "New launch" / "Edit launch" with the muted line "Saved launches open a new session with one click." (new keys `wizard_title_new`, `wizard_title_edit`, `wizard_subtitle`; `wizard_header` removed). Title and Type are a plain group with no section heading. The connection or program section follows, then a separate "After exit" section with its segmented control and a muted hint (`exit_behavior_hint`). Save/Cancel, validation errors under fields and optional markers were already in place and are unchanged. The four keys are in all 10 locales.
- `context_menu.rs`: Command: [Kill], Edit, Rename, Duplicate, separator, Remove (danger). Folder: Create launch, Create folder, separator, Rename, separator, Delete (danger). Background: Create launch, Create folder. The popup height counts the separators so it still fits the window.
- Kit `MenuItem` gained `.danger(bool)` (status-error text; default off, so the command palette is unaffected).
- No shortcut labels: none of F2, Del or Enter exist for launches. There is no "Run" action in `ContextMenuAction`, so none was added. Cancel already existed (`WizardCancel`).

### Tests (written first)

`iced_test`: Save sends `WizardSave`; Cancel sends `WizardCancel`; SSH type sends `WizardSelectCommandType`; new-launch heading and subtitle are shown; every context menu item (command, running command, folder, background) sends its `ContextMenuAction`. Reducer tests untouched.

### Not verified

The look (app not run); the danger color and separators are not covered by a test.

## Comments

Regression checklist steps to walk by hand: 10 (create a launch via the form), 11 (edit a launch), 12 (rename), 13 (duplicate), 14 (remove and delete), 8 (kill a running launch from the menu), 9 (SSH launch edit with type switch), 20 and 21 (light theme and a non-English language show the new strings). Check screenshots in both themes: menu separators and red Remove/Delete, form heading and "After exit" hint.
