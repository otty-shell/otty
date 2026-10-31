# 05 — Meta badge and hover actions

Status: ready-for-agent
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: 04
Spec: [../spec.md](../spec.md)

## Details

- On the first line of each finished command block, right-aligned, draw the badge from spec AC5 in the terminal's small text size: `✓ 0.4 s` (mark `status-ok`, time `text-muted`) or `✕ exit 1 · 8.4 s` (mark and code `status-error`, time `text-muted`). Running blocks show nothing; `Unknown` shows nothing.
- If the prompt text reaches the badge, the badge gets the block's fill behind it so it stays readable; it never moves terminal text.
- On hover the badge is replaced by two small actions: Copy command and Copy output (existing `BlockCommand::Copy…`); the existing right-edge action button merges into this place instead of being a second control.
- Terminal theme settings get the badge colors (`block_meta`, `block_ok`, `block_error`) from `text_muted()`, `status_ok()`, `status_error()`.

## Tests first

- The badge text for each state (pure function; uses ticket 03).
- Geometry: the badge is right-aligned and clipped to the block; with no room it is hidden.

## Done when

- Spec AC5, AC6; screenshots in both themes: success, failure, hover.
