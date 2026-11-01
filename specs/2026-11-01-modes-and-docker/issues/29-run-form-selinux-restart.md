# 29 — Run form: SELinux `:z` switch, Restart starts at No

Status: ready-for-agent
Type: task
Class: C
Model: Sonnet 5.5 (claude-sonnet-5-5)
Blocked by: 14
Spec: [../spec.md](../spec.md)

## What

Spec Decision 20 (Run form points). Tests first.

- `otty-docker`: a pure check whether SELinux is enabled, from the contents
  of `/sys/fs/selinux/enforce` (exists → enabled; read once per form); the
  `RunRequest` volume mount gains a relabel flag that appends `z` to the bind
  options (`src:dst:z` or `src:dst:ro,z`).
- Run form: for a row whose source is a folder on this computer, show the
  switch "Let the container read this folder (:z)" only when SELinux is
  enabled; off by default; hint that it changes the SELinux labels of the
  folder's files. Named volumes never show it.
- Restart starts at No (check that it does today; keep a test).

All new text goes through `i18n` in the ten locales (spec Decision 17). Any missing visual piece goes into `otty-ui-kit` first with a gallery section and an `iced_test`. Tests first. Run fmt, clippy, tests, `cargo deny check` and compare `cargo llvm-cov` with the merge base before the commit.

## Done when

- Crate tests: the bind string for each combination; the SELinux check on
  plain inputs.
- Reducer / view model tests: switch shown only with SELinux and a folder
  source; off by default; carried into the request.
