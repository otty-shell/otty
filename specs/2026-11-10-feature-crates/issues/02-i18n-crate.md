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

### 2026-11-10: what the translations crate holds

`crates/app/i18n` (`otty-app-i18n`) depends only on `serde` and
`serde_json`. Copied from `app/src/i18n/`, with `pub(crate)` made `pub`:

- `key.rs` whole (`Key` and the test-only `Key::ALL`);
- `locales/*.json` whole, still embedded with `include_str!`;
- `mod.rs` as `lib.rs`: `Locale`, the active locale (`current_locale`,
  `set_locale`), `t`, `text_in`, `text_locale_tag` and every text builder
  such as `with_error`, `launch_failed` or `containers_action_failed`, with
  their tests. `text_in` moved above the private functions to follow the
  module order rule.

Left out:

- `detect_system_locale`: it read the system locale with `sys-locale`. The
  crate takes the language as a value. The pure part,
  `locale_from_system_preference(Option<&str>)`, is now public, so the
  binary (issue 05) calls it with `sys_locale::get_locale().as_deref()` and
  passes the result to `set_locale`. Reading the language from the settings
  file stays in the binary too.
- The test `given_a_confirm_dialog_switch_then_its_label_fits_the_dialog`:
  it measures a `Switch` from `otty-ui-kit`, which brings iced. It belongs
  with the containers feature that shows that dialog.
- `Mode::name_key`: only the modes widget uses it, and that widget goes to
  the binary. The binary maps `Mode` to `Key::ModeTerminal` or
  `Key::ModeContainers` there, so this crate does not depend on
  `otty-app-domain`.
