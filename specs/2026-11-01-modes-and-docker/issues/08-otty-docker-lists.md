# 08 — `otty-docker`: list and map containers, images, volumes, networks

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 07
Spec: [../spec.md](../spec.md)

## What

Plain OTTY types and list calls (spec Decisions 7, 10). Tests first.

- `Container` (id, name, state enum, status text, exit code, image, ports, created, compose project, mounts, networks, command), `Image` (id, tags, size, created, how many containers use it), `Volume` (name, driver, mountpoint, created, labels, containers that use it), `Network` (id, name, driver, scope, subnet, gateway, containers, created, built-in).
- Mapping from bollard models is a pure function per type, tested on fixture JSON captured from Docker and from Podman. Where Podman fills a field differently (for example image container counts), compute it from the container list.
- "Used by" for images and volumes comes from the container list, not from extra calls.
- No time or size formatting here; the app formats.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Mapping tests for every type on Docker and Podman fixtures, including dangling images, anonymous volumes, built-in networks, exited containers with and without an error code.
