# 11 — Container actions, confirm dialogs, notices

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Container actions of spec Decision 11. Tests first. Screen: `DockerMode`
(tweaks `confirm`, `notice`).

- `otty-docker`: start, stop, restart, pause, unpause, remove (with force) and prune stopped.
- Inspector buttons by state — running: Shell (ticket 15), Restart, Stop, Pause, Remove; paused: Unpause, Stop, Remove; restarting: Stop, Remove; exited or created: Start, Remove.
- The same actions in a context menu on right click on a row (kit `Menu`).
- Remove and Prune ask first (kit `Dialog`); Remove of a running container has the switch "Stop it first", on by default.
- While an action runs its buttons are disabled. A failure shows a kit `Notice` with the engine's message; success reloads the containers list.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer tests: each action's effect, the button set per state, the confirm flow (cancel and confirm), failure shows a notice, dismiss hides it.
