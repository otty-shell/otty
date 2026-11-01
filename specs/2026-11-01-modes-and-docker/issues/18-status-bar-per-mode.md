# 18 — Status bar per mode

Status: ready-for-agent
Type: task
Class: B
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 09
Spec: [../spec.md](../spec.md)

## What

Docker mode status bar as on the boards: engine and version, the counts of
the open resource ("6 containers, 3 running", "7 images, 3.0 GB", "4
volumes", "5 networks"), "<other engine> also found" when there are two, and
the sessions button. Terminal mode unchanged.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- View model tests for each resource and for one and two engines.
