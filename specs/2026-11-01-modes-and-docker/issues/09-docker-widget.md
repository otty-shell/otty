# 09 — `docker` widget: engines, tree, tables, selection, filter, Refresh

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 02, 06, 08
Spec: [../spec.md](../spec.md)

## What

New widget `app/src/widgets/docker/` (spec Decisions 5, 8, 10). Tests first.
Screens: `DockerMode`, `DockerImages`, `DockerVolumes`, `DockerNetworks`.

- State: engines and their lists with a load status per list; the open place (engine, resource, compose filter); selected row per resource (by id); filter text.
- Entering Docker mode runs discovery (ticket 07) and loads all lists (`Task::perform`); Refresh reloads all lists. Nothing runs while another mode is active.
- Left panel: one root per engine ("Docker 27.3", "this computer"), children Containers ("N running"), Images, Volumes, Networks with counts. Compose projects come in ticket 17.
- Main area: title, engine chip, filter field, the create button (disabled until tickets 12 and 14), Refresh, and the kit `DataTable` with the columns of Decision 10 and the badge tones.
- Right panel: kit `InspectorHeader` of the selected row (title and status); tabs and buttons come in tickets 11–13.
- Age text ("2 min", "2 h", "3 d", "2 w", "1 mo", "1 y") and size text are small tested helpers.
- Selection survives a reload when the id is still there and moves to the first row otherwise.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: open each resource, filter, reload keeps or drops the selection, a failed list shows its error and keeps the other lists.
- View model tests: columns and badge tones per resource; counts in the tree; age and size helpers.
