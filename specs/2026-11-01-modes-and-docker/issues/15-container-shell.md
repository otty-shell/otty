# 15 — Shell into a container as a terminal tab

Status: resolved
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

## Answer

Shell is the first button of a running container (inspector and row menu). Tests were written first and seen failing, then the code.

- `otty-docker`: `CliPresence::docker_path()` and `podman_path()` give the full path of the found command (needed when the app starts from the macOS Finder with a short `PATH`). `podman_path()` is the `podman` binary, else a `docker` command that is really Podman (`podman-docker`). `CliPresence` is no longer `Copy`, so `Discovery::cli()` returns a reference; nothing outside the crate used it. `fixtures::discovery_with_cli` builds a search with commands at given paths.
- App: `RowAction::Shell` is first in `container_actions(Running)`. `action_flow::act` emits `DockerEffect::OpenShell { title, program, args }` with title "shell <name>", the full CLI path of the engine kind, and args `exec -it <id> sh`; nothing happens when the CLI was not found. `events/docker.rs` opens a command tab on this computer (`TabsIntent::OpenCommandTab`, the quick launch path) and then sends `ModesIntent::Select(Mode::Terminal)`.
- Disabled case: `action_buttons` disables Shell and sets `ActionsViewModel::hint` when the engine CLI is missing.
- i18n: 4 keys in ten locales.

Tests: CLI paths (install folder, wrapper as Podman, `podman` preferred); reducer effects for Docker, Podman, the wrapper, a Docker CLI ignored for Podman, missing CLI, stopped container; view model enabled and disabled with hint; the handler opens the tab and then switches to Terminal mode.

Deviations: the tab uses `ExitBehavior::Close`; no live engine check was run. Manual check: Shell on a running container opens a working `sh` in Terminal mode; Shell is disabled with a hint when the CLI is missing.
