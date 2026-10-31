# 06 — Rerun action

Status: needs-triage
Type: task
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 05
Spec: [../spec.md](../spec.md)

## Details

- Add Rerun to the hover actions and the block context menu. It writes the block's `cmd` and Enter to the session.
- Enabled only when the session's last block is an idle prompt (no running command, not full-screen); otherwise hidden.
- Locale strings in all 10 locales.

## Tests first

- The enable rule: idle prompt → enabled; running command, full-screen app, block without `cmd` → disabled.
