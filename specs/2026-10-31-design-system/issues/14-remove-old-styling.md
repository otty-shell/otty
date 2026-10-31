# 14 — Remove old styling code

Status: ready-for-agent
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 07, 08, 09, 10, 11, 12, 13
Spec: [../spec.md](../spec.md)

## Details

- Delete what the kit replaced: `app/src/style.rs` (keep `thin_scroll_style` only if no kit equivalent; then move it to the kit), `app/src/components/primitive/icon_button.rs` and `menu_item.rs`, local style functions in widget views.
- `StyleOverrides` in `app/src/theme.rs`: remove if nothing uses it after the migration.
- Check AC6 and AC7 with `grep`; list any local size constants that stay, each with a doc comment.
- Run the full check list from AGENTS.md and compare coverage with the merge base (AC12).

## Done when

- AC6, AC7, AC11, AC12 hold.
