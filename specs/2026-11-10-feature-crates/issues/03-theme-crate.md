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

### 2026-11-10: what the theme crate holds

`crates/app/theme` (`otty-app-theme`) depends on `iced`, `otty-ui-kit`,
`otty-ui-term` (the terminal palette type and `parse_hex_color`) and
`otty-app-domain` (for `Mode`). Copied, with `pub(crate)` made `pub`:

- `theme.rs` whole with its tests: `ColorPalette`, `BuiltinTheme`,
  `AppTheme`, `ThemeProps`, `ThemeManager`. The `ignore` doc example became a
  real one, and `ThemeManager` got a `Default`, which clippy asks of a public
  type with `new()`.
- `icons.rs` whole, with the `include_bytes!` paths now going four folders up
  to `assets/`. It also has `mode_icon(Mode)`, which does what the old
  `Mode::icon` did (issue 01 left it out of the domain crate).
- `fonts.rs`: `TerminalFonts` and `FontsConfig`. Their fields are private now,
  read with `font()`, `size()` and `terminal()`, since nothing sets them.

What reuses the kit instead of a copy:

- The UI colors are the kit's `UiPalette`, the fonts its `fonts::MONO` and
  `typography::TERM`, as in the old app. The theme crate has no copy of them.
- `apply_text_locale` was left out. It only passes the i18n language tag to
  the kit's `fonts::set_text_locale`, and it needs `otty-app-i18n` (issue 02),
  which this crate does not depend on. The binary (issue 05) can call the kit
  with the tag from i18n, and should bring the test
  `han_letters_take_the_shapes_of_the_language_without_a_restart` with it.
- The test `hangul_stays_whole_in_each_weight_with_the_font_asked_for` was
  left out: it checks only the kit's fonts and `register_variable_cjk_faces`,
  nothing of this crate. It stays in the old app.

`app/src/geometry.rs` is now `otty_ui_kit::geometry` (`anchor_position`,
`menu_height_for_items`) with its six tests. The kit has its own placement of
a menu under a button (`overflow_menu`, `floating_list`), which is different
math, so nothing was merged. `cargo tree -p otty-ui-kit` names no `otty-app`
crate. The old app is unchanged.
