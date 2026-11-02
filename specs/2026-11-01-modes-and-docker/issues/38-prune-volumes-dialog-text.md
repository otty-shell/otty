# 38 — Volume prune dialog says what will really be removed

Status: resolved
Type: bug
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 37
Spec: [../spec.md](../spec.md)

## What

Found while syncing the canvas (ticket 20). The Prune unused volumes dialog
always says "They and all data in them are deleted, named ones too"
(`tpl_docker_prune_volumes_text`), also with "Also remove named volumes" off,
when ticket 23 keeps named volumes. The dialog misstates a destructive action.

Two texts in all ten locales, picked by the switch, in
`app/src/widgets/docker/action_model.rs`:

- Off: "Anonymous volumes no container uses: {count}. They and all data in
  them are deleted. Named volumes stay. This cannot be undone."
- On: "Volumes no container uses: {count}. They and all data in them are
  deleted, named ones too. This cannot be undone."

The count already follows the switch. Tests first. Run fmt, clippy, tests,
`cargo deny check` and compare `cargo llvm-cov` with the merge base before the
commit.

## Done when

- View model test: each switch position gives its text with the right count.
- The i18n test fills both templates in every locale.

## Answer

The dialog now picks its text by the switch. Off uses the new
`tpl_docker_prune_anonymous_volumes_text` (`Key::TplDockerPruneAnonymousVolumesText`)
and says named volumes stay. On keeps `tpl_docker_prune_volumes_text`
unchanged. Both exist in all ten locales, and the "named volumes stay"
sentence is in every off text. `Key::ALL` is now 409 long.

- The view model test checks the exact English text for both switch
  positions, with counts 1 and 2.
- The i18n fill test includes the new template for every locale.
- Not run against a live engine.
