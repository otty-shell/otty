# 14 — Run container form

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09, 12
Spec: [../spec.md](../spec.md)

## What

The form of spec Decision 18. Tests first. Screen: `DockerRun` (tweaks
`state`, `invalid`).

- Opens from "Run container" above the Containers table and from "Run" on an image (image filled in). Built from kit `FormPage`, `FormSection`, `TextField`, `TokenField`, `KeyValueField`, `Select`, `Segmented`, `Switch`.
- Form state and checks are pure and tested: image not empty, ports 1–65535, container paths absolute, a host path source exists. "Remove when it stops" forces Restart to No and disables the other choices.
- `otty-docker`: a `RunRequest` type mapped to bollard's create body (tested), then create and start.
- Flow: pull if missing and the switch is on (progress in the footer), create, start, show Containers with the new container selected. An engine error shows a `Notice` at the top and keeps every field.
- Right panel: what the image sets on its own (ports, command, entrypoint, workdir) from image inspect, or "Not on this computer yet".

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests: each check, the auto remove and restart rule, the form → request mapping (ports, env, binds and volumes, read only, network, restart policy, auto remove, command), the success and error flows.
- Manual: run nginx with a port and a bind mount on Docker and on Podman; `docker inspect` shows the same settings.
