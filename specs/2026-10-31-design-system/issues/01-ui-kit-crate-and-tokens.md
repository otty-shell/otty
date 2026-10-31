# 01 — Create otty-ui-kit crate with size, spacing, radius and type tokens

Status: ready-for-agent
Type: task
Model: Opus 5.5 (claude-opus-5-5)
Blocked by: —
Spec: [../spec.md](../spec.md)

## What

Add the workspace crate `otty-ui-kit` at `crates/ui/kit` and move every
non-color design token into it, so later tickets can build components on it.

## Details

- `Cargo.toml` copies the shape of `crates/ui/tree/Cargo.toml`; depends only on `iced` from the workspace. No new external dependencies.
- The workspace already lists `"crates/ui/*"`; check that the new crate is picked up without editing the root `Cargo.toml`.
- Modules, one responsibility each:
  - `spacing.rs` — `SPACE_1` … `SPACE_7` (4, 6, 8, 12, 16, 24, 36).
  - `radius.rs` — `RADIUS_XS` 4, `RADIUS_SM` 6, `RADIUS_MD` 8, `RADIUS_LG` 10.
  - `size.rs` — `CONTROL_SM` 24, `CONTROL_MD` 28, `CONTROL_LG` 34, `ROW` 30, `TAB` 34, `TITLEBAR` 38, `STATUSBAR` 24, `RAIL` 48, `RAIL_BUTTON` 44, `PANEL` 260, `TREE_INDENT` 18.
  - `typography.rs` — a `TextStyle` with size and line height for `title`, `heading`, `body`, `body-strong`, `small`, `caption`, `term`, `term-small`, `kbd`, values from `tokens.json` `type.groups`, and the weight of each.
- Every public item has a one-line doc comment that repeats the token's usage note from the design system.
- `app` depends on `otty-ui-kit`. `app/src/layout.rs` re-uses the kit constants where they match (`BUTTON_SIZE_COMPACT` → `CONTROL_SM`, `BUTTON_SIZE_REGULAR` → `CONTROL_MD`, `BUTTON_SIZE_RAIL` → `RAIL_BUTTON`, `BUTTON_RADIUS_ROUNDED` → `RADIUS_SM`). Values of existing screens do not change in this ticket.
- No tests: these are plain constants (AGENTS.md, infrastructure rule).

## Done when

- `cargo build -p otty-ui-kit` and `cargo build -p otty` pass.
- The app looks exactly as before.
