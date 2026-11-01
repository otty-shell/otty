# 30 — Row press opens the right panel; inspector tabs become a select when narrow

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 28
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20. Tests first.

- In Docker mode, pressing a table row (a plain left press) opens the right
  panel when it is closed. The row "⋯" button and the right click do not.
  Use the sidebar widget's existing toggle path; no new layer.
- Kit `inspector_tabs`: when the tabs do not fit the given width, render a
  kit `Select` with the same choices instead (built with `responsive`).

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Reducer / events tests: row press with the panel closed opens it; with it
  open nothing changes; "⋯" and right click leave it closed.
- Kit `iced_test`: wide → tabs, narrow → select, both emit the same message.
