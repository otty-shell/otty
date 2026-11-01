# 24 — Kit: sortable DataTable, link cell, row ⋯ button, collapsing Toolbar, icon tooltips

Status: ready-for-agent
Type: task
Class: A
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 02
Spec: [../spec.md](../spec.md)

## What

Kit parts for spec Decision 19. Each gets a gallery section in both themes and
`iced_test` tests, styled only with `UiPalette` and kit tokens.

- `DataTable`:
  - Sortable headers: a column can be sortable; pressing its header emits a
    message with the column index; the sorted column shows an up or down
    arrow. The table does not sort rows itself (the app passes rows in order).
  - Link cell: `TableCell::link(label, message)`, drawn like text in the
    accent color with a hover underline; pressing it emits its message and
    does not select the row.
  - Row "⋯" button: an optional last column with a small "⋯" icon button per
    row; pressing it emits `(row index, point)` for a menu and does not select
    the row. Rows without actions can hide it.
- `Toolbar`: a row of items (a flexible field such as the filter, then icon
  buttons with labels for tooltips and menus). When the row is too narrow, the
  buttons that do not fit move into a trailing "⋯" menu, in order; the field
  shrinks first down to a minimum width. Built with `responsive`; no layout
  logic in the app.
- Icon buttons with tooltips: check `IconButton`; it must take a tooltip text
  and a danger tone. Use it for the toolbar and the inspector actions.
- New stroke icons in `assets/svg/` in the style of the existing ones (24×24,
  round caps): play/start, stop, restart, pause, resume, trash/remove,
  shell/terminal, download/pull, prune (broom), plus/create, more (⋯) if
  missing. Original drawings only; no third-party icon set is copied. Since
  the kit never bundles icons (callers pass `svg::Handle`), the app wires
  them; the gallery may load them from `assets/svg`.

All new text goes through `i18n` in the ten locales (spec Decision 17). Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- `iced_test`: header press emits its column; link press emits its message
  and not a row press; row "⋯" emits index and point and not a row press;
  toolbar overflow puts the last buttons into "⋯" at a narrow width and
  pressing an overflow item emits its message; icon button tooltip text is
  present.
- Contrast tests for the link color and the sort arrow on `surface-canvas`
  and `surface-selected`, both themes.
