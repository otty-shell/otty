# 38 — Volume prune dialog says what will really be removed

Status: ready-for-agent
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
