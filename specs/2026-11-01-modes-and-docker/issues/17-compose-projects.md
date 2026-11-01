# 17 — Compose projects in the tree

Status: resolved
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Spec Decision 10.

- Under each engine root, "Compose projects" with one child per `com.docker.compose.project` label and a container count. Research for Podman: which label `podman compose` / `podman-compose` sets; note it in Comments and support it too.
- Choosing a project shows Containers filtered to it, with a "compose: <name>" chip that clears the filter (see `DockerMode`).

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests: grouping, counts, filter and its chip.

## Answer

Done as view of existing data (Class B), tests first.

- `Place` now carries an optional Compose project (`Place::compose`, `compose_project()`).
- Under Containers, `model::tree_rows` adds a "Compose projects" row (count of projects) and one child per project (count of containers), sorted by name. No projects, or lists still loading: no row. The group row opens the unfiltered containers table, so opening it keeps the containers table.
- `types::shown_containers` applies the name filter and the project; the table and `select_shown_row` both use it, so a row press picks the row that is shown.
- `TableViewModel::compose_chip` holds the "compose: <name>" text; `view/main_area.rs::header_row` shows it after the engine chip. Its cross sends `DockerIntent::ClearComposeProject`, which reopens the engine's containers.
- Kit: `Chip` got `on_clear(label, message)` (a cross inside the chip, tooltip label), with iced_test tests and a gallery entry.
- i18n: `DockerComposeProjects`, `TplDockerComposeChip`, `DockerShowAllContainers` in all ten locales (`Key::ALL` is 254).
- Limit of the tests: the shared fixtures have only one Compose project with one container per engine, so grouping with two containers is tested on a cloned fixture container; two different project names are not covered.

## Comments

Research answer (from ticket 08): Docker Compose, `podman compose` and `podman-compose` all set `com.docker.compose.project`, so no extra label is read. The Podman fixture carries it too.
