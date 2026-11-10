# 01 Domain crate

Status: needs-triage
Blocked by: —

## What to do

Create the `crates/app/` folder and the first helper crate, `otty-app-domain`,
in `crates/app/domain`. It has no iced and no I/O.

Copy into it from the old app (the old files stay where they are):

- `app/src/domain/` (`Mode`, `ExitBehavior`, `QuickLaunch`);
- `app/src/commands.rs` (`AppCommand`);
- the shortcut names and their labels from `app/src/shortcuts.rs`
  (`AppShortcut`), without the iced key matching, which goes to the binary in
  issue 05;
- `app/src/shell_session.rs`.

The types one feature hands to another (`TabLaunch`, `TabMachine`,
`WizardTabInit`) are added later, by the issue of the first feature that needs
each of them.

Add the workspace members for `crates/app/`. Do not touch the `app` member.

Prove how clippy finds its settings for the coming feature crates: put a
`clippy.toml` with the feature bans (see the spec, "Clippy bans") in
`crates/app/features/`, add a throwaway crate under it that calls
`std::fs::read`, and check that clippy fails on it. Then remove the throwaway
crate. Write in this issue's comments whether one shared file works or each
feature crate needs its own copy.

## Done when

- `otty-app-domain` builds without iced in its dependency tree.
- The copied tests pass in the new crate.
- The clippy check above is done and its result is written down.
- The old `app` is unchanged.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments

### 2026-11-10: clippy lookup for the feature crates

One shared file works. `crates/app/features/clippy.toml` holds the feature
bans and the root thresholds. A throwaway crate,
`crates/app/features/clippy-probe`, called `std::fs::read`. Both
`cargo clippy -p otty-app-clippy-probe -- -D warnings` and the full
`cargo clippy --workspace --all-targets --all-features -- -D warnings` failed
on it with:

```
error: use of a disallowed method `std::fs::read`
  = note: a feature touches no files; the binary does it in effects/
```

The root `clippy.toml` bans `std::fs::read` too, but with the reason "in app,
files live in services.rs or storage.rs". The note above is the reason from
the features file, so clippy read the nearest file and not the root one. The
throwaway crate and its workspace member were removed; the shared file stays.
Since the nearer file replaces the root one, it repeats the root thresholds.

### 2026-11-10: what the domain crate holds

`crates/app/domain` (`otty-app-domain`) depends only on `serde`
(`serde_json` for tests). Copied, with `pub(crate)` made `pub`:

- `exit_behavior.rs`, `quick_launch.rs` and `commands.rs` whole, with their
  tests, and doc comments added to the accessors that had none;
- `mode.rs` without `name_key` and `icon`;
- `shortcuts.rs`: `AppShortcut`, `ALL`, `for_mode_position`, `command`,
  `label`, `label_on` and `Platform`;
- `shell_session.rs`: only `shell_name`.

Left out:

- `WizardTabInit` (in the old `quick_launch.rs`): a cross-feature type, added
  by the issue that first needs it.
- `Mode::name_key` needs `i18n::Key`, which comes with `otty-app-i18n`
  (issue 02). `Mode::icon` needs the icon bytes from `otty-app-theme`, which
  has iced, so it can not live here; the theme or the binary can map a mode
  to its icon.
- The shortcut keys (`AppShortcut::keys`, `ShortcutKeys`) and the key
  matching (`shortcut_for_key_press`, `terminal_bindings`) are iced types and
  go to the binary in issue 05. Because the old labels were built from those
  iced keys with `otty_ui_kit::shortcut_text`, `label_on` here is a plain
  table of texts and returns `&'static str` instead of `String`. Issue 05
  should keep the old check `labels_are_built_from_the_bindings` in the
  binary, so the label table and the keys can not drift apart.
- `ShellSession`, `shell_session_options` and their tests: they use
  `otty_ui_term::settings::{LocalSessionOptions, SessionKind}`, and
  `otty-ui-term` brings iced and tokio. `shell_session_options` also reads
  `HOME`. They stay with the binary; `shell_name` got its own tests here.
