# 25 — Docker tables: Compose column, no Compose tree group, no "this computer", host:container ports

Status: resolved
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 24
Spec: [../spec.md](../spec.md)

## What

Spec Decision 19, first, third and fourth points.

- Remove the "Compose projects" group and its children from the engine tree
  (ticket 17). Keep `Place`'s compose filter and the chip with its clear
  button.
- Containers table: a **Compose** column (after Image) with the project name
  as a kit link cell; pressing it opens the compose filter for that project.
  Empty for containers without a project.
- Remove "this computer" from engine roots in the tree and the engine chip
  from the table header. Remove now-unused i18n keys.
- Ports: `8080:80/tcp` in the table and the inspector; the Run form's port
  row separator shows `:` instead of the arrow (volumes keep the arrow).

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests: tree rows without the Compose group; the Compose column
  and link intent; root labels without "this computer"; port text.
- `iced_test`: pressing a Compose link opens the filter; the chip clears it.

## Answer

Tests were written first (the port, tree, column, link and separator tests
failed or did not compile against the old code), then the code.

- Compose column after Image in the containers table (title reuses
  `Key::DockerFieldCompose`, "Compose" in all locales). A container with a
  project shows `TableCell::link(project)`; without one, an empty muted cell.
  The table sets
  `.on_link_press(|row, _| DockerIntent::OpenComposeProject(row))`.
- New intent `OpenComposeProject(index)`: the reducer asks
  `DockerState::open_compose_project_of_row`, which looks up the shown
  (filtered, compose-limited) container at `index` and opens
  `Place::compose(engine, project)`. A row without a project, another
  list, or no engine does nothing. The compose chip with its clear button is
  unchanged.
- The tree has no "Compose projects" group or project rows. The Containers
  row stays selected while a compose filter is on. `compose_projects()` and
  the i18n key `DockerComposeProjects` (all 10 locales, `Key::ALL` now 400)
  are removed.
- Engine roots in the tree have no "this computer" note, and the table
  header has no engine chip (`TableViewModel::engine_label` is gone). The
  `ThisComputerNote` key stays: the Terminal session list still uses it.
- Ports read `8080:80/tcp` in the table and the inspector overview
  (`ports_text`). The Run form port rows use the new kit
  `MappingField::separator(":")`; volumes keep the arrow.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.42%
(63742 lines, 8654 missed) to 86.43% (63784 lines, 8658 missed).
