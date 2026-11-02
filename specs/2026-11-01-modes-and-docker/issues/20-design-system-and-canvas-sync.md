# 20 — Update the design system and the canvas to what was built

Status: ready-for-human
Type: task
Class: —
Model: Opus 5.5 (claude-opus-5-5), owner review
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

- Add the new kit components to the design system artifact (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3).
- Update the `Docker mode` boards on the canvas wherever the built app differs.
- Mark `V4ModeSwitch` and `V4ModesMany` as superseded, or change their Ctrl 1–5 labels to the Decision 6 table.
- The owner reviews before publishing.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Design system and canvas match the app; the owner has approved them.

## Answer

The canvas is published. The design system update is prepared but not published: it waits for the owner's review. This ticket stays `ready-for-human` until the owner approves both.

### Canvas

Published to https://claude.ai/artifact/5QkFQQUDrqMuqVSFyBZCh6 as version 40 (id `1793620693-0e3a`). The owner had made no hand edits since the last version (the file hashes matched).

- `DockerMode`, and through it `DockerModesMenu`, `DockerImages`, `DockerVolumes`, `DockerNetworks`:
  - No mode name, "this computer" or engine chip. The tree has Containers, Images, Volumes and Networks only.
  - The header has two rows: the title with the compose chip, then the filter and icon buttons.
  - The tables sort as in the app, with the arrow after the title and running containers first. The Containers table has Compose and Ports links (`8080:80/tcp`). Every row has a "⋯" menu.
  - The inspector has icon actions with Remove in red, Overview links, keys above values, Logs with Follow and "N new lines", and Inspect as a tree with the two switches and Copy JSON.
  - Pressing a row opens a closed right panel.
  - A new `inspector` tweak (230, 320, 150) shows two columns and tabs as a select.
- `DockerRun`: `:` between ports, the SELinux switch on a folder volume, and Restart starting at No. A new `selinux` tweak hides the switch.
- `DockerNoEngine`, and through it `DockerEngineStopped`, `DockerPodmanOff`, `DockerNoAccess`: no mode name, and the "looked for" line as built.
- `V4ModeSwitch`, `V4ModesMany`: titles now say they are superseded and that their Ctrl 1–5 labels are old.

Not drawn or not matched:

- The boards are 1440 wide, so they do not show the toolbar collapsing or the narrow table and tree of ticket 36.
- Port links show their URL as a tooltip and open nothing. Prune, Refresh and Copy JSON do nothing.
- Prune dialogs are not drawn.
- The boards keep their sample containers, and two engines, instead of the screenshot fixtures.
- The result was not rendered, following the artifact's rule.

### Design system

The update waits in `scratchpad/ds-update/project/` of the session, in the design system's own layout. It has 13 new cards, 7 extended cards, notes on 2 more, the bundle, stylesheet and types, the brand book, 9 size tokens and 16 icons. The live design system is unchanged, at version `1793541467-b234`. **It needs the owner's review before publishing.**

`scratchpad/ds-update/SUMMARY.md` was not written, because the session's tools blocked that report file; the summary went to the owner in the session's report instead.

For the owner to decide:

1. Publish the update as it is, or ask for changes.
2. Keep `RailButton` and the `rail` tokens, marked as not in the app, or remove them. The update keeps them because older canvas screens use them.
3. Upload the 16 new icons into the Icons group. The update assumes yes.
4. Install the new design system copy on the canvas afterwards, as a separate change. Until then the Docker boards draw the new parts with `project/docker-mode.css`.

### Found on the way

The Prune dialog of Volumes always says "They and all data in them are deleted, named ones too" (`tpl_docker_prune_volumes_text`), even with "Also remove named volumes" off, when named volumes are kept. Not changed here; it needs its own fix in all ten locales.
