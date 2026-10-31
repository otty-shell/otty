# 01 — Add iced_test, characterization tests and the regression checklist

Status: ready-for-agent
Type: task
Class: C (tests only, no behavior change)
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

Pin today's behavior before the layout changes, so later tickets show at once
when they break something.

- Add `iced_test = "0.14"` as a `[dev-dependencies]` entry of `crates/ui/kit` and `app` (owner approved, spec Decision 7). Run `cargo deny check`; record any license or duplicate warning in Comments.
- Characterization tests (unit tests next to the reducers, no view code), one per flow the redesign touches:
  - tabs: open terminal, settings, launch editor and error tabs; activate; close the active and a background tab; which tab becomes active after a close;
  - sidebar: toggle visibility, resize, collapse below the threshold (`SIDEBAR_COLLAPSE_WORKSPACE_RATIO`) — these move to the new state in ticket 07 and must keep their numbers;
  - explorer: follows `shell_cwd_for_active_tab` when the active tab changes;
  - quick launch: run a command launch and an SSH launch up to the secret prompt; cancel the prompt.
  Reuse tests that already exist; add only the missing ones.
- One `iced_test` smoke test on a kit component (e.g. `Button` click gives its message) to prove the setup, in both crates.
- Write `regression-checklist.md` in this spec folder: the manual flows of spec AC1, each as a numbered step with the expected result. Later tickets tick it in their Comments.

## Done when

- `cargo test --workspace --all-features` passes with the new tests; `cargo deny check` passes.
- `regression-checklist.md` exists and was walked once on the current build (note results in Comments).
