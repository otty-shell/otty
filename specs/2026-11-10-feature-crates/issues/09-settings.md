# 09 Settings

Status: needs-triage
Blocked by: 08

## What to do

Move `app/src/widgets/settings` into `otty-app-settings`, following "Moving a
feature" in the spec. Reading and saving settings (`storage.rs`) go to
`effects/settings.rs` in the binary.

Behaviour to keep (from the old app):

- Settings is a wide dialog over every mode, not a tab;
- Save keeps the dialog open;
- closing drops unsaved edits; Esc and a click on the dimmed area outside both
  close it;
- the gear and Ctrl+, do nothing while it is open;
- a failed read, a failed save or an invalid file becomes a notification;
- the chosen theme and language reach the window and new terminals.

## Done when

- The behaviour above works in the new app.
- `otty-app-settings` reads and writes no files itself.
- The steps and checks in "Moving a feature" are done.

## Comments
