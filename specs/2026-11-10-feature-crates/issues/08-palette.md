# 08 Palette

Status: needs-triage
Blocked by: 07

## What to do

Move `app/src/widgets/palette` into `otty-app-palette`, following "Moving a
feature" in the spec.

- Picking an entry returns `CommandPicked(AppCommand)`. The binary runs it
  through `dispatch/command.rs`, the same path as a shortcut.
- Whether a command can run is checked by the palette's `reduce` when an entry
  is picked, from data in its `Ctx`, as in the old app.
- The palette's own shortcut keeps its query; the palette opens above a
  Containers dialog and under the SSH secret prompt.

## Done when

- Opening, searching and running commands from the palette works as in the old
  app.
- `otty-app-palette` depends only on helper crates and `otty-ui-*`.
- The steps and checks in "Moving a feature" are done.

## Comments
