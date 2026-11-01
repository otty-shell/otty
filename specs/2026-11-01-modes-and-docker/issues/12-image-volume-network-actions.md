# 12 — Image pull, volume and network create, remove and prune

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Actions for images, volumes and networks (spec Decision 11). Tests first.
Screens: `DockerImages`, `DockerVolumes`, `DockerNetworks`.

- Pull image dialog: name and tag (default `latest`), a progress line built from the pull stream, Cancel stops it. Progress aggregation over layers is a pure, tested function in `otty-docker`.
- Image remove asks first, with the switch "Force" when a stopped container uses it; Prune dangling images asks first.
- Create volume dialog (name, driver `local`); remove asks first; Prune unused volumes asks first.
- Create network dialog (name, driver `bridge`); remove asks first; Prune unused networks asks first.
- Remove is disabled with a hint for a volume or network a container uses and for `bridge`, `host`, `none`.
- Failures show a `Notice`; success reloads the affected list.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests for every action and dialog; tests for the disabled-Remove rules; pull progress aggregation tests.
