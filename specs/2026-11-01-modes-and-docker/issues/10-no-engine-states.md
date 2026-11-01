# 10 — No engine states, open links, copy command

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 06, 07
Spec: [../spec.md](../spec.md)

## What

When discovery finds no engine, Docker mode shows why and what to do
(spec Decision 9). Screens: `DockerNoEngine`, `DockerEngineStopped`,
`DockerPodmanOff`, `DockerNoAccess`.

- Kit `EmptyCard` in the main area with the title, text, command and buttons of each case; the "looked for" line under it; left panel text "No engine found." and the like; empty inspector.
- Check again reruns discovery without a restart.
- Copy writes the command with the iced clipboard.
- `open_url` in `app/src/services.rs`: `xdg-open` on Linux, `open` on macOS, through `std::process::Command`; a failure is logged.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests for each case (texts, command, links).
- `iced_test`: Install, How to fix, Copy and Check again emit their messages.
- Manual: each case on a real machine (no engine; `systemctl stop docker`; `systemctl --user stop podman.socket`; a user outside the docker group).
