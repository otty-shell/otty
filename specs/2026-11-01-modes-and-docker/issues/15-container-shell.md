# 15 — Shell into a container as a terminal tab

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 11
Spec: [../spec.md](../spec.md)

## What

Shell of spec Decision 13.

- Shell opens a new terminal tab running `docker exec -it <id> sh`, or `podman exec -it <id> sh` for a Podman engine, through the same path command launches use to open tabs. Tab title "shell <name>", grouped under this computer.
- Then switch to Terminal mode.
- When the engine's CLI is not on `PATH` (known from ticket 07), the button is disabled with a hint.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: the open-tab effect has the right program and arguments for Docker and Podman; the mode switches; the disabled case.
