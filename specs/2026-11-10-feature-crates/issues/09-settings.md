# 09 Settings

Status: needs-triage
Blocked by: 08

## What to do

Move `app/src/widgets/settings` into `otty-app-settings`, following "Moving a
feature" in the spec. Reading and saving settings (`storage.rs`) go to
`effects/settings.rs` in the binary.

Behaviour to keep (from the old app):

- Settings is a wide dialog over every mode, not a tab;
- Save keeps the dialog open;
- closing drops unsaved edits; Esc and a click on the dimmed area outside both
  close it;
- the gear and Ctrl+, do nothing while it is open;
- a failed read, a failed save or an invalid file becomes a notification;
- the chosen theme and language reach the window and new terminals.

## Done when

- The behaviour above works in the new app.
- `otty-app-settings` reads and writes no files itself.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the settings feature crate

**`otty-app-settings`** (`crates/app/features/settings`) is a copy of
`app/src/widgets/settings` without `storage.rs` and `errors.rs`; the old
widget is unchanged (`git diff --stat baf3a27 -- app` is empty). It depends
on `iced` (with `svg`), `log`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme`, `otty-ui-kit`, `serde` and `serde_json`; for tests on
`iced_test` and `otty-app-testing`. Its public API:

- `Settings` with `new(saved)`, `is_open`, `saved() -> &SettingsData`,
  `terminal_editor()` (the saved editor a pressed file opens with; issues 10
  and 11 read it) and `has_unsaved_edits` (public now, because the binary's
  tests check it);
- `SettingsData`, `LanguageSetting` (its old `resolve()` is now
  `fixed() -> Option<Locale>`: the binary reads the system language),
  `SettingsLoad` with the pure `parse(text)` and `missing()`, and
  `SettingsLoadStatus`. Turning the file's text into settings is data work,
  so it stays in the feature with its tests; only the file reads and writes
  moved to the binary;
- `SettingsProblem` (`ReadFailed`, `FileInvalid`, `SaveFailed`) with
  `text()`;
- `impl Feature` with `Ctx<'a> = ()`: `reduce` reads no outside data, so
  there is no `ctx.rs`;
- `impl Overlays`: the open dialog as `Overlay::dialog`, `DIALOG_WIDE`, with
  `on_backdrop_press(Close)`.

```rust
pub enum SettingsIntent {
    Open, Close, ReloadLoaded(SettingsLoad), ReloadFailed(String), Save,
    SaveCompleted(SettingsData), SaveFailed(String), Reset,
    LanguageChanged(LanguageSetting), ShellChanged(String),
    EditorChanged(String), EqualizePanesToggled(bool),
    ThemeSelected(BuiltinTheme),
}

pub enum SettingsOutcome {
    /// The dialog opened: it takes the keyboard from every widget.
    Opened,
    /// Read the saved settings; answer ReloadLoaded or ReloadFailed.
    ReadSaved,
    /// Write these settings; answer SaveCompleted or SaveFailed.
    WriteSaved(SettingsData),
    /// Saved or read again: the window and new terminals take their
    /// language, theme and shell.
    Applied(SettingsData),
    /// The dialog closed: in Terminal mode the active pane gets the keys.
    Closed,
    /// Tell the user about a failure.
    Failed(SettingsProblem),
}
```

`SettingsEffect`, the `SettingsEvent` wrapper, the `Task::done` hops and the
`focus::nothing()` of the reducer are gone. The old reducer called
`i18n::set_locale` itself; now `Applied` carries the settings and the binary
sets the language. The copied reducer tests were rewritten to check the
returned outcomes and run first against the copied `Task` reducer (21 build
errors: no `==` and no `is_empty` on a `Task`, no `SettingsLoad::parse` and
`missing`), then `reduce` changed to return `Vec<SettingsOutcome>`. The
in-flight edit tests were folded into one test over every edit.

**The dialog check** (the first dialog) in the binary:

- `guards.rs`: `Overlay::Settings`, kind `Dialog`, on top in `top_overlay`
  (over the notifications list, the palette and the menus, in either mode);
  Esc sends `SettingsIntent::Close`, which drops unsaved edits;
  `dialog_guard(dialog, event)` returns only `Allow` or `Ignore`: the
  dialog's own answers (`is_dialog_answer`: its fields, choices, Save, Reset
  and `Close`) and what passes every dialog (`passes_every_dialog`: the
  pointer moving over the window, a hover over a Files row, the window
  buttons and title bar, the resize grips) pass; shortcuts, header buttons
  (the gear, the bell, the title bar search), `PaletteIntent::Open`, a
  toast's cross, mode and sidebar presses and other keys are ignored, and a
  menu under the dialog stays open;
- `update.rs`: after Esc, a dialog on top runs `dialog_guard` before the
  menu checks;
- the backdrop exception: the dialog view sends `Close` on a press on the
  dimmed area, and `Close` is one of its answers, so it passes the check;
  the other dialogs (12, 14) give no backdrop message;
- `view.rs`: the Settings layer right over the notifications list, in the
  main area (the window buttons stay usable under it); the resize grips are
  off while it is open; the dialog's dimmed area lies over the toasts.

**How the binary wires it:**

- `AppEvent::Settings(SettingsIntent)` and `App::settings`;
- `dispatch/settings.rs`: `Opened` → `take_keyboard()`, `ReadSaved` →
  `effects::settings::read`, `WriteSaved` → `effects::settings::write`,
  `Applied` → `apply` (the language with `effects::locale::apply`, which also
  tells the fonts, as `apply_text_locale` did; the theme in
  `app.theme_manager`; the shell through
  `effects::shell_integration::integrate`, which answers `ShellIntegrated`),
  `Closed` → nothing until issue 11, `Failed(problem)` → a toast with
  `problem.text()`. Applied comes before Failed in the outcomes, so a broken
  file's text is in the language it brings, as in the old app;
- `dispatch/command.rs`: `AppCommand::OpenSettings` → `SettingsIntent::Open`,
  so the gear, the shortcut and the palette entry open it;
- `effects/settings.rs` is rewritten: `read_at_start` (synchronous in
  `App::new`, as before), `read` and `write` (`Task::perform`, answers as
  settings intents), the atomic write and the test path. The old
  `SavedSettings` and the repeated `from_json` rules are gone; `App::new`
  and `App::with_saved(saved: SettingsData, pinned)` take the feature's data;
- `effects/startup.rs` writes the shell integration through the same
  `integrate`;
- `TempDir` moved from the binary's `effects/temp_dir.rs` to
  `otty-app-testing::temp_dir`, because the feature's drawing test (Reset
  looks like Cancel) keeps its reference hash in one too.

**Gaps resolved:** the Settings layer and the dialog check (the old
`a_dialog_ignores_shortcuts_app_buttons_and_keys`, `a_dialog_lies_over_the_toasts`
and `the_bell_does_nothing_while_settings_is_open` are ported);
`AppCommand::OpenSettings` from the gear, the shortcut and the palette
entry; the theme and the language at run time; the storage replaces the
old `effects/settings.rs`; the saved editor is exposed; a Files hover passes
under the dialog. There were no `dead_code` marks for issue 09; the notes
in `update.rs`, `view.rs`, `guards.rs`, `dispatch/command.rs` and
`dispatch/explorer.rs` are resolved. No "issue 09" note is left in
`crates/app/`.

**Left for later issues:**

- 10 and 11: opening a pressed file with `app.settings.terminal_editor()`
  (note on the `OpenFile` arm).
- 11: on `Applied`, send the theme colors to the open terminals (old
  `TerminalWorkspaceIntent::ApplyTheme`, note on `apply`); new terminals
  read the theme from `app.theme_manager`, the shell from
  `app.shell_session` and the equalize-panes switch from
  `app.settings.saved()`; `Closed` focuses the active pane in Terminal mode
  (old `closing_settings_gives_the_pane_its_keys_in_terminal_mode`); typed
  terminal keys kept out under the dialog; terminal reports added to
  `passes_every_dialog`. The old scrolling tests
  `settings_opened_again_starts_at_its_top` and
  `settings_built_again_keeps_its_scrolling` need the frame helper and the
  window size of the main area, which come with the terminal screen.
- 12: the SSH prompt over Settings (`top_overlay`, its layer, its answers in
  `is_dialog_answer`, the old tests that the prompt lies over Settings and
  takes the press on the dimmed area), the Launches hovers in
  `passes_every_dialog`.
- 14: the Containers dialogs in `is_dialog_answer` (and the old test that a
  press on their dimmed area keeps them open).

**Tests.** Copied and ported: the state, types, view model, reducer and
dialog view tests of the widget; the file tests of the old `storage.rs` and
`effects/settings.rs` (the parse half in the feature, the file half in the
binary); from the old binary the dialog check, Esc, the gear and the
shortcut while open, the bell and shortcuts under the dialog, the dialog
over either mode, the window buttons and the grips under it, the press on
the dimmed area, the dialog over the toasts. New: `SettingsProblem` texts,
the dialog overlay kind, the saved editor, the dispatch of each outcome,
the palette entry, a dialog over a menu keeps the menu. `otty-app-settings`
has 76 tests and 3 doc tests, `otty-app` 409 tests (376 before),
`otty-app-testing` 3 doc tests (2 before). The whole workspace runs 4517
tests, all passing.

**Checks.** `cargo tree -p otty-app-settings -e normal` has no `notify`;
`tokio` comes only through `otty-ui-term` → `otty-app-theme`
(`cargo tree -i tokio`), no direct `tokio`. A planted `std::fs::write` in
the feature failed clippy with `use of a disallowed method std::fs::write`
and the note `a feature touches no files; the binary does it in effects/`;
it was removed. `cargo +nightly fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, `cargo deny check`
(only the 3 known yanked-crate warnings), `cargo test --workspace
--all-features` and `cargo build -p otty-app` pass. `cargo llvm-cov` was not
run, as asked. The window was not opened (no display for the agent).

**For the owner to decide.**

- A failure is `Failed(SettingsProblem)`, whose text is built when shown,
  not `Failed(String)` as in the other features: the settings applied in
  the same update may change the language first.
- Applied settings now write the shell integration files in the background
  (`Task::perform`); the old app wrote them inside the update before it
  sent the theme on.
- Parsing the settings text stays in the feature (`SettingsLoad::parse`),
  so `SettingsData`, `SettingsLoad` and `SettingsLoadStatus` are public.
- `TempDir` now lives in `otty-app-testing`, shared by the binary's effects
  tests and the feature's drawing test.
- The binary's tests apply settings with English fixed (`test_settings()`),
  so a test never switches the process language to the system's.

### 2026-11-10: the binary reads SHELL

The settings feature no longer reads `SHELL`. The binary reads it once in
`App::new`, next to `HOME`, falling back to `/bin/bash` as before, and
keeps it in `App::default_shell`. It gives the value to the parse of the
settings file (`SettingsLoad::parse(text, default_shell)`,
`SettingsLoad::missing(default_shell)`, and the defaults after a failed
read at start) and to the feature as `Settings::new(saved, default_shell)`,
which saves an emptied shell as it. The defaults are now
`SettingsData::defaults(default_shell)`; `SettingsData` has no `Default`
outside the crate's own tests. Reset still restores the saved settings,
not the defaults. The `#[expect(clippy::disallowed_methods)]` exception is
gone: no feature crate reads the environment, and the clippy ban on
`std::env::{var, var_os, vars}` holds for all of them.
