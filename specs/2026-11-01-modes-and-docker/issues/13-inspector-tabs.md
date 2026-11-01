# 13 — Inspector: Overview, Logs (follow), Inspect

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 03, 09
Spec: [../spec.md](../spec.md)

## What

Inspector tabs of spec Decision 12. Tests first. Screen: `DockerMode`.

- Tabs: containers have Overview, Logs, Inspect; the other resources Overview, Inspect (kit `InspectorTabs`).
- Overview: kit `KeyValueList` with the fields shown on the boards; empty fields are left out.
- Logs: a subscription keyed by engine and container id, alive only while Docker mode is active, the Logs tab is open and that container is selected. Starts with the last 500 lines, then follows. At most 5000 lines are kept. Kit `LogView`.
- Inspect: the engine's inspect JSON, pretty printed, loaded when the tab opens.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Tests: the logs subscription exists only in the right conditions and stops on tab change, selection change and mode change; the line cap; Overview fields per resource; Inspect loads once per selection.
