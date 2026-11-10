# 02 Translations crate

Status: needs-triage
Blocked by: 01

## What to do

Create `otty-app-i18n` in `crates/app/i18n` by copying `app/src/i18n/` from the
old app. It has no iced and no I/O. Both the features (texts of notifications,
labels) and the binary use it.

If the translations read the system locale or a settings value, the reading
stays in the binary; the crate gets the language as a value.

## Done when

- `otty-app-i18n` builds without iced in its dependency tree.
- The copied tests pass in the new crate.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments
