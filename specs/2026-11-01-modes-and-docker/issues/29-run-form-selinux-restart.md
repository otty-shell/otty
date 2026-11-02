# 29 — Run form: SELinux `:z` switch, Restart starts at No

Status: resolved
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

## Answer

Tests were written first and seen failing (compile errors for the new API,
then green after the code).

- **`otty-docker`:** `VolumeMount::with_relabel(bool)` and `is_relabeled()`.
  The bind text joins the options with a comma: `src:dst`, `src:dst:ro`,
  `src:dst:z`, `src:dst:ro,z`. A named volume ignores the flag. New
  `is_selinux_enabled()` (`selinux.rs`): reads `/sys/fs/selinux/enforce`;
  the pure part `is_enabled_from(Option<&str>)` says "enabled" for any
  contents (permissive mode still has labels) and "off" when the file is
  missing.
- **Run form (`run_form.rs`):** `RunForm::new` takes `is_selinux_enabled`;
  `run_flow::open` reads it once per form. `VolumeRow` has the switch
  (`RunEdit::VolumeRelabel`), off by default. `offers_relabel(index)` is true
  only with SELinux and a folder source (`/…` or `~…`). The request sets the
  flag only when the switch is on and `offers_relabel` is true, so a source
  edited from a folder to a volume name cannot leak a `z`.
- **View:** `VolumeRowViewModel` has `offers_relabel` and `is_relabeled`;
  the row shows the switch "Let the container read this folder (:z)" with
  the hint "Changes the SELinux labels of the folder's files." under its
  inputs. New kit piece `MappingRow::detail(element)` (an element under the
  inputs and above the error) with two `iced_test`s and a gallery row.
- **Restart** already started at No (`RestartPolicy::No` in `RunForm::new`);
  the tests `a_new_form_pulls_missing_images_and_does_not_restart` and the
  view test `restart_starts_at_no` keep it.
- **i18n:** `DockerRunRelabel`, `DockerRunRelabelHint` in the ten locales;
  `Key::ALL` is 403. Nothing became unused.

Tests: crate (bind text for every ro/z combination, named volume ignores
`z`, create body, SELinux check on plain inputs); form (off by default,
offered only for a folder with SELinux, request carries it, never for a
named volume or without SELinux); view model (same cases, on/off); view
(switch and hint shown only for a folder with SELinux, press sends
`VolumeRelabel(0, true)`, restart starts at No); kit (`detail` position and
press). Tests that need SELinux on or off use the test-only
`RunForm::set_selinux`, so they do not depend on the machine.

Coverage (lines, `cargo llvm-cov --workspace --all-features`): 86.71%
(65309 lines, 8681 missed) on `92739b8` -> 86.82% (65614 lines, 8648
missed).
