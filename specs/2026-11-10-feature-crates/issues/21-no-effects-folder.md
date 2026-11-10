# 21 No effects folder in the binary

Status: needs-triage

## Why

The binary `otty-app` had a folder `src/effects/` with four files. The folder
existed only because the binary's `clippy.toml` bans background work, files
and processes, and the files there lifted the ban at their top. Most of them
were thin:

- `shell_integration.rs` only wrapped
  `otty_app_shell::integrated_shell_session` in `Task::perform`. The io
  crates of the features return their own `Task`s; the shell helper did not.
- `host_name.rs` and `startup.rs` were the startup work split over two
  files.
- `locale.rs` held `apply(language)`, four lines that set the language and
  tell the fonts. That is no I/O. Most of the file was tests of feature views
  in every language (the launch editor, the Run form), which lived in the
  binary only because they switch the language and the fonts of the whole
  process and need to run in a copy of the test program.

## What to do

1. Move the background wrapper into `otty-app-shell` as
   `integrate(shell_path, dir) -> Task<ShellSession>`, like the io crates.
   The binary maps it to `AppEvent::ShellIntegrated` at start and when the
   shell changes in Settings.
2. Merge `host_name.rs` and `startup.rs` into `crates/app/otty/src/startup.rs`:
   the shell session, then the host name, then the first tab. It becomes the
   only file of the binary that lifts the clippy bans. Update the comments
   and ban reasons of `crates/app/otty/clippy.toml`.
3. Move `apply` to `crates/app/otty/src/language.rs` without a ban lift, with
   only the test about `apply` itself. Move the other tests into the tests of
   the feature crates they test (quick launch and containers), driving the
   feature through its `reduce` and `main_area`. Put the shared helpers
   (running a test alone in a copy, loading the kit's fonts) into
   `otty-app-testing`.
4. Delete `effects/` and fix the references, the module docs and the spec.

The behaviour of the app does not change.

## Done when

- `crates/app/otty/src/effects/` is gone and `grep -rn effects crates/app`
  shows nothing stale.
- A `Task::perform` planted in another file of the binary fails clippy.
- Every moved test runs and passes in its copy ("1 passed").
- All checks from AGENTS.md pass, and `app/` is unchanged.

## Comments

### 2026-11-10: the folder is gone

**Where things went.**

- `otty_app_shell::integrate` (`crates/app/shell/src/integration.rs`) writes
  the integration files in the background with `Task::future` and ends with
  the `ShellSession`. `otty-app-shell` now depends on the workspace `iced`;
  `Cargo.lock` gains no new package, only the new edge. A plain wrapper, so
  it has no test of its own; the startup test of the binary runs it.
- `crates/app/otty/src/startup.rs` holds `run` (shell session, host name,
  first tab, in this order) and `read_host_name` (nix `gethostname`), with
  the test `the_shell_session_and_host_name_go_to_the_app_before_the_first_tab`.
  It is the only file of the binary with `#![allow(clippy::disallowed_methods)]`.
  `dispatch/settings.rs` calls `otty_app_shell::integrate` directly.
- `crates/app/otty/src/language.rs` holds `apply` and its test
  `han_letters_take_the_shapes_of_the_language_without_a_restart`. It has no
  ban lift.
- `otty-app-quick-launch` (`lib.rs` tests) got
  `the_optional_marker_of_the_launch_editor_is_in_each_language`. It opens
  the editor with `WizardInitializeCreate` and `WizardSelectCommandType`, the
  intents the binary sends after New launch, and draws `main_area`.
- `otty-app-containers` (`lib.rs` tests) got
  `the_read_only_label_of_the_run_form_reads_whole_in_every_locale`,
  `the_command_placeholder_reads_whole_at_the_smallest_width_in_every_locale`
  and `the_optional_marker_is_in_the_language_of_the_run_form`, with
  `can_measure` and `hangul_stays_whole`. The Run form opens through
  `reduce` on the fixture Docker lists, as the binary's helper did, and is
  drawn through `main_area`. The tests set the language and the fonts'
  language the way `apply` does.
- `otty-app-testing` got `alone::is_alone` (start the test in a copy of the
  test program and check "1 passed" there) and `fonts::load_kit_fonts`. The
  `Command::new` of the copy lives in `alone.rs`, with the allow at its top,
  so the feature tests need no allow. The crate now depends on `iced_test`
  and `otty-ui-kit`, both already in `Cargo.lock`; it stays a dev-only
  crate.
- The binary's `clippy.toml` says background work starts in `startup.rs`, or
  in an io crate or a helper crate such as `otty-app-shell`. The module docs
  of `main.rs`, `message.rs`, `dispatch/mod.rs` and `testing/` and the spec
  (the binary's list, the clippy bans, where `Task::perform` lives and who
  changes no state) say the same. The binary README never named `effects/`.

**Checks.**

- A `Task::perform` planted in `language.rs` failed clippy with "use of a
  disallowed method `iced::Task::perform`" and the new reason; it was
  removed.
- Each moved test, run in its copy with `OTTY_TEST_ALONE=1` and
  `--nocapture`, printed "1 passed", and no language was skipped on this
  computer. A wrong assertion planted in the quick launch test made the
  normal run fail, so a test that silently does nothing would be caught.
- `cargo +nightly fmt --all`: clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`:
  clean.
- `cargo deny check`: advisories, bans, licenses and sources ok; only the
  three known yanked-crate warnings (`clipboard-win`, `core2`, `spin`).
- `cargo test --workspace --all-features`: 6322 passed, 0 failed, 5
  ignored (three tests that need a live Docker or Podman engine and two doc
  tests of `otty-libterm`, all ignored before this issue).
- `git diff --stat 56864b7 -- app`: empty.
- `cargo llvm-cov` was not run for this issue.
