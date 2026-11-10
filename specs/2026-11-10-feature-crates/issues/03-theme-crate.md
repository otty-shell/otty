# 03 Theme crate

Status: needs-triage
Blocked by: 01

## What to do

Create `otty-app-theme` in `crates/app/theme` by copying from the old app:

- `app/src/theme.rs` with `ThemeProps`;
- `app/src/icons.rs`; fix the `include_bytes!` paths to `assets/` for the new
  folder;
- `app/src/fonts.rs`.

Check first what `otty-ui-kit` already has (it has `palette`, `typography` and
`fonts`). If a piece of the app theme is only a copy of the kit, use the kit
instead of copying it.

Copy `app/src/geometry.rs` (where a menu opens) into `otty-ui-kit`. It is plain
interface math. The old app keeps its own `geometry.rs`.

## Done when

- `otty-app-theme` builds, and `otty-ui-kit` has the menu geometry.
- `otty-ui-kit` still depends on no crate under `crates/app/`.
- The copied tests pass in their new crates.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments
