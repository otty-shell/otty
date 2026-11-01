# 02 — Kit: DataTable, StatusBadge, Notice; TreeRow count tone

Status: ready-for-agent
Type: task
Class: A
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

New kit components (spec Decision 15). Visual reference: `ot-table`,
`ot-status`, `ot-banner` in the design system bundle and the `DockerMode`,
`DockerImages`, `DockerVolumes`, `DockerNetworks` boards.

| Component | Shape |
|---|---|
| `DataTable` | Header row (32 tall, 12px muted text, `line` under it) and body rows (34 tall, `surface-raised` line under each). Columns have a title, alignment (left, right for numbers) and a width share. Cells: plain, mono, muted mono, muted, number (tabular, right), badge (`StatusBadge`). Hover `surface-header`, one selected row `surface-selected` with `text-strong`. Long text ends with an ellipsis. Vertical scroll. A message row when there are no rows. |
| `StatusBadge` | Dot plus text; tones ok, warn, error, neutral (`text-muted`, dot `text-faint`). |
| `Notice` | `ot-banner`: `surface-danger`, `text-danger`, error icon, text, dismiss button. Only the error tone is needed now. |

- `DataTable` is generic over `Message` only. Rows are plain cell lists, not a row trait.
- Messages: row pressed (index), row right-pressed (index and cursor point for a context menu), notice dismissed.
- `TreeRow` gains a count tone (default, failed, waiting, done) if it has none yet; existing callers keep the default.
- Gallery sections in both themes, including a selected row, every cell kind and the empty message.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first for Class C (AGENTS.md). Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: row press emits its index; right press emits index and point; dismiss emits its message.
- Contrast tests: every badge tone on `surface-canvas` and on `surface-selected`, both themes; notice text on `surface-danger`.
