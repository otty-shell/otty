# 20 — Update the design system and the canvas to what was built

Status: ready-for-agent
Type: task
Class: —
Model: Opus 5.5 (claude-opus-5-5), owner review
Blocked by: 19
Spec: [../spec.md](../spec.md)

## What

- Add the new kit components to the design system artifact (https://claude.ai/artifact/7zMwx63cZnNmTUeTL1FQF3).
- Update the `Docker mode` boards on the canvas wherever the built app differs.
- Mark `V4ModeSwitch` and `V4ModesMany` as superseded, or change their Ctrl 1–5 labels to the Decision 6 table.
- The owner reviews before publishing.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Design system and canvas match the app; the owner has approved them.
