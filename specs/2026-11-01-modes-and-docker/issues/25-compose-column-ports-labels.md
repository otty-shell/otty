# 25 — Docker tables: Compose column, no Compose tree group, no "this computer", host:container ports

Status: ready-for-agent
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
