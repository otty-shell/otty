# 03 — Kit: KeyValueList, InspectorHeader, InspectorTabs, LogView

Status: ready-for-agent
Type: task
Class: A
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New kit components (spec Decision 15). Visual reference: `ot-inspector`,
`ot-insp-*` in the design system bundle, `.logv` in `project/docker-mode.css`
and the inspector on the `DockerMode` board.

| Component | Shape |
|---|---|
| `KeyValueList` | Two columns (96 / rest), keys muted, values mono, long values wrap, 8px gap. |
| `InspectorHeader` | Mono title (14), a `StatusBadge` line, a wrapping row of small buttons, an optional hint line under the buttons. |
| `InspectorTabs` | Underlined text tabs: selected tab `text-strong` with a 2px `accent` line. |
| `LogView` | Mono 11.5 / 18, `surface-chrome`, `radius-sm`, selectable text, wraps long lines, scrolls; while following it stays at the bottom when lines are added. |

- Messages: tab selected. Buttons are passed in as elements (kit `Button`).
- Gallery sections in both themes; `LogView` shows a follow toggle in the gallery only.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: a tab press emits its message; header buttons emit theirs.
- `LogView` keeps the bottom in view after lines are appended while following, and does not jump when not following.
