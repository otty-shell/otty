# 17 — Compose projects in the tree

Status: ready-for-agent
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
