# 14 Containers: inspector, forms and actions

Status: needs-triage
Blocked by: 13

## What to do

Move the rest of `app/src/widgets/containers` into `otty-app-containers`: the
child modules `inspector`, `run`, `create` and `actions`. They become plain
inner modules of the crate.

This issue covers:

- the inspector in the right panel, with its tabs, inspect tree, links and log
  follow toggle; the parent keeps the open inspector tab;
- the Run container form in the main area;
- the create dialogs for volumes and networks; creating one still locks Prune;
- the row menu and the confirm dialogs;
- pulling images, with the pull handle kept in the run and create state;
- their Docker calls in `effects/containers.rs`.

## Done when

- Everything in Containers mode works as in the old app.
- `otty-app-containers` still has no `bollard` and no `tokio`.
- The steps and checks in "Moving a feature" are done.

## Comments
