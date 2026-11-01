# 19 — OTTY Light pass and screenshots

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 10–18
Spec: [../spec.md](../spec.md)

## What

Check every new screen in OTTY Light and fix contrast or token issues in the
kit. Take the screenshots of spec AC12 in both themes and attach them to the
PR next to the `Docker mode` canvas boards.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Screenshots of: Terminal mode with the switcher, the open All modes menu, each Docker table, each inspector tab, the Run form, each no-engine case — Dark and Light.
