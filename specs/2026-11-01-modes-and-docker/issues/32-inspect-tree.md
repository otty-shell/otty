# 32 — Inspect as a readable tree with Raw JSON and Copy

Status: ready-for-agent
Type: task
Class: C
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 28
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20 (Inspect). Tests first.

- Kit: a tree view for structured data (collapsible sections, indented key
  and value rows, numbered list rows, long values wrap), with a gallery
  section and `iced_test`. It takes plain rows built by the app; no JSON
  inside the kit.
- App: build the tree from the inspect JSON (`serde_json::Value`, already a
  dependency of the app through otty-docker? — check; if the app needs
  `serde_json` it is already a workspace dependency of the app). Top-level
  keys become sections in a sensible order (State, Config, HostConfig,
  NetworkSettings, Mounts, then the rest); empty values (null, "", [], {})
  are hidden until "Show empty" is on.
- A "Raw JSON" switch shows today's JSON view; "Copy JSON" copies it with the
  iced clipboard.

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Pure tests: JSON → rows for objects, nested objects, lists, empty values,
  section order.
- Reducer tests: section open/close state per selection, Raw switch, copy
  effect.
- `iced_test`: sections toggle; Raw shows the JSON; Copy emits its intent.
