# 05 Binary with the window frame

Status: needs-triage
Blocked by: 01, 02, 03

## What to do

Create the binary `otty-app` in `crates/app/otty` and the helper crate
`otty-app-slots` in `crates/app/slots`.

The binary opens a window with the frame and no features yet:

- `main.rs`, `app.rs`, `Message`, `update.rs` with the input checks;
- the overlay rules from `guards.rs` (layer order, Esc, outside click, focus
  after close; see the spec, "Overlays");
- `view.rs` with the frame and the one layer order;
- `screens/terminal.rs` and `screens/containers.rs` with empty places;
- the frame widgets `chrome`, `sidebar` and `modes` as plain modules;
- `subscription.rs` and turning iced key presses into `AppCommand`;
- `dispatch/` and `effects/` with the startup I/O (shell integration files,
  host name, saved theme).

`otty-app-slots` starts with only the `Feature` trait, `Overlay` and
`MainContent`. The frame widgets implement `Feature`, which gives the trait its
first real implementations. The place traits come with the first feature that
fills each place.

The frame widgets switch to outcomes here. Their `reduce` returns
`Vec<Outcome>`, and `dispatch/` reads them. No `Task::done` goes from one widget
to another. Rewrite their copied tests to check the outcomes first, then change
the code.

`app.rs` and `update.rs` will grow with every feature. That is expected: they
are the wiring.

## Done when

- `cargo run -p otty-app` opens a window with the header, both side panels, the
  status bar and the mode switch. Switching modes, resizing the panels and focus
  mode work as in the old app.
- No `Task::done` is sent from one widget to another.
- The old `app` is unchanged and still builds and ships.
- fmt, clippy, `cargo deny check`, the tests and the coverage comparison from
  AGENTS.md are run, and their results are reported.

## Comments

### 2026-11-10: left for this issue by issues 01–03

The helper crates left these pieces to the binary, because they need iced,
`otty-ui-term` or I/O:

- From 01 (domain):
  - turn an iced key press into `otty_app_domain::keys::ShortcutKeys` (a
    digit-row key becomes `Digit1`..`Digit5`, any other key its lower-case
    character) and find the `AppShortcut` whose `keys_on` equals it;
  - build the terminal widget's key bindings;
  - `ShellSession` and `shell_session_options` (they use `otty-ui-term` and
    read `HOME`), with their tests from `app/src/shell_session.rs`.
- From 02 (i18n):
  - read the system language and call
    `set_locale(locale_from_system_preference(sys_locale::get_locale().as_deref()))`;
    reading the language from the settings file also stays here;
  - map `Mode` to `Key::ModeTerminal` / `Key::ModeContainers`, with the old
    test from `app/src/domain/mode.rs`.
- From 03 (theme):
  - `apply_text_locale`: call `otty_ui_kit::fonts::set_text_locale` with the
    i18n language tag, and bring the test
    `han_letters_take_the_shapes_of_the_language_without_a_restart`;
  - the fonts are read through accessors now: `fonts.terminal().size()`.

### 2026-11-10: the binary with the frame and the slots crate

**`otty-app-slots`** (`crates/app/slots`) depends on `iced`,
`otty-app-theme` and `otty-ui-kit`. It holds only:

- `Feature` with `Intent`, `Outcome`, `Ctx<'a>`,
  `reduce(&mut self, intent, ctx) -> Vec<Outcome>` and `subscription()`
  (none by default);
- `Overlay` (content plus `OverlayKind::Menu` or `Dialog`);
- `MainContent<'a, Message, Screen>`: `Screen { id, content }` or
  `ScrollsOnItsOwn`, with `into_element`. The old `ScreenId` named the
  screens of every feature (one of them a containers type), so each feature
  now names its screens with its own type; the screens of two features never
  share scrolling, because their types differ;
- `focus::NOTHING`, from `app/src/focus.rs`.

No place traits yet. Its docs explain the socket, with examples that run as
doc tests.

**`otty-app`** (`crates/app/otty`, binary `otty-app`) copies the frame of the
old app. The old `app/` is unchanged (`git diff --stat 5c176b4 -- app` is
empty). Modules:

- `main.rs` (window setup, fonts), `app.rs` (`App`, `new`, `with_saved`,
  `title`, `theme`, `subscription`, `update`, `view`), `message.rs`
  (`Message::Input/Internal` around `AppEvent`, whose variants are the three
  frame widgets, `Command`, `ShellIntegrated`, `HostNameRead`, `Keyboard`,
  `Window` and `ResizeWindow`);
- `update.rs` (input checks), `guards.rs` (the overlay rules for the header
  menus and the All modes menu: `top_overlay`, `context_menu_guard`, Esc,
  `command_ctx`), `keys.rs` (iced key press → `AppShortcut` through
  `otty_app_domain::keys`), `view.rs` (the frame and the one layer order),
  `screens/{terminal,containers}.rs` (the places, empty but the status bar),
  `subscription.rs`, `layout.rs`, `shell_session.rs`;
- `dispatch/{mod,chrome,sidebar,modes,command}.rs`: each reads the outcomes
  of its widget, one call per arm;
- `effects/{startup,shell_integration,host_name,settings,locale,modes}.rs`:
  the shell integration files and the host name (`Task::perform`, chained in
  this order), the saved theme, language and shell, the system language with
  the text locale of the fonts, and the pinned modes file;
- `chrome/`, `sidebar/`, `modes/`: the frame widgets as plain modules.

The frame widgets implement `Feature` (`Ctx = ()`) and return outcomes:

- `ChromeOutcome`: `Command(AppCommand)` for the header buttons (the button
  → command mapping moved from the old effects into `reduce`),
  `ToggleMaximize`, `MinimizeWindow`, `CloseWindow`, `StartWindowDrag`.
- `SidebarOutcome`: `MainAreaResized` (old `SyncTerminalGridSizes`),
  `PanelEdgeDragged` (old `QuickLaunchResetInteractionState`),
  `HeaderMenuOpened` (the frame takes the keyboard; old `focus::nothing()`
  in the reducer), `HeaderMenuClosed`. Its focus note frames are its
  `Feature::subscription`.
- `ModesOutcome`: `ModeChanged(Mode)`, `SavePinned(Vec<Mode>)`,
  `MenuOpened`, `MenuClosed(Mode)`, `PinnedSaveFailed(String)`. The save
  answer comes back as `ModesIntent::PinnedSaved(Result<(), String>)`, so the
  user text is built in `reduce` and the effect only logs.

The copied reducer tests were rewritten to check the returned outcomes and
run first against the copied `Task` reducers (they failed to build), then
the reducers changed. The same red step was done for the key matching and
for the settings and modes file readers. A chrome button now runs its
command in the same update. `grep -rn "Task::done" crates/app/otty/src`
finds it only in the test helper `testing/task_outputs.rs` and its tests;
`App::new` returns the startup work itself, so the old `IcedReady` hop is
gone.

Tests copied and ported: the three reducers, states, view models, views
(`iced_test`), the overlay rules and input checks from `guards.rs` and
`update.rs` that concern the header and modes menus, the key matching from
`shortcuts.rs`, `ShellSession`, shell integration, the modes file, the
startup order, `every_mode_has_its_own_name` and
`han_letters_take_the_shapes_of_the_language_without_a_restart` (it runs in a
copy of the test program, as before). `labels_are_built_from_the_bindings`
was not brought: since issue 01 the labels are built from the same
`keys_on` table the key matching reads, and
`every_shortcut_is_found_from_its_own_key_press` checks that table against
iced key presses. `otty-app` has 302 tests, `otty-app-slots` 5 doc tests;
the whole workspace runs 4199 tests, all passing.

**Gaps left for the feature issues.** The items that only a later feature
reads or sends are kept and marked with
`#[cfg_attr(not(test), expect(dead_code, reason = "..."))]` (or a plain
`expect` when tests do not use them either). The reason names the issue, and
the mark fails the build once the item is used, so each issue removes its
marks when it wires the item.

- 06 notifications: the toasts layer in `view.rs` and the list layer; the
  bell's open state and unseen count in the header (`false`, `0`);
  `AppCommand::OpenNotifications`; `ModesOutcome::PinnedSaveFailed` and the
  modes file load problem (`_modes_problem` in `App::new`) and the settings
  read problems as notifications; closing the list in `close_all_menus`;
  the list in `top_overlay` and the resize grip check.
- 07 explorer: the right panel of Terminal mode; its file watching in
  `subscription.rs`.
- 08 palette: its layer; `AppCommand::OpenPalette`; the palette shortcut
  under the open palette in `filter_input`; closing it in
  `close_all_menus`; `top_overlay`.
- 09 settings: its layer and dialog check; `AppCommand::OpenSettings`; the
  theme change at run time (old `ThemeChosen`) and the language change with
  `apply_text_locale`; its storage replaces `effects/settings.rs`, which now
  reads only the theme, the language and the shell at start.
- 10 tabs: the Sessions section (`ToggleSessions`, `is_sessions_open`, the
  sidebar's `left_panel` view, not copied yet), the New session header menu
  (`HeaderMenuOpen`, `HeaderMenu`, `OpenHeaderMenu::{menu, cursor}`,
  `open_header_menu`, and the header menu layer under the modes menu,
  `view_header_menu_overlay` with `screen_size_from_window` and the menu
  anchor), the header menu choices in `filter_input`, `ModesIntent::TabShown`,
  `ShellSession::name`, the first tab chained after the startup work, the
  title of the active tab in the header, `has_active_tab` in `command_ctx`,
  `NewTerminal`/`CloseSession`, the machine and session count in the status
  bar.
- 11 terminal: the main area of Terminal mode; `ShellSession::session` and
  the terminal settings from the theme and `fonts.terminal()`; the terminal
  key bindings built from `AppShortcut::keys_on`; the binding path in
  `as_command`; terminal input kept out under an overlay; `MainAreaResized`,
  `AppEvent::Window` (new window size), `pane_grid_size` and
  `side_panels_ratio`; focusing the active pane on `HeaderMenuClosed`,
  `MenuClosed(Terminal)` and `ModeChanged(Terminal)`; `has_focused_pane`;
  `SplitRight`/`SplitDown`; the pane context menu; pane and folder in the
  status bar; the terminal subscriptions.
- 12 quick launch: the Launches section (`ToggleLaunches`,
  `is_launches_open`), its header menu, the second half of `NewFolder` after
  `RevealLaunches`, `NewLaunch`, `PanelEdgeDragged` and
  `SidebarState::{is_resizing, cursor}`, the inline edit check, the Launches
  context menu, the SSH secret prompt layer and dialog check.
- 13 containers: the places of Containers mode (the status bar is the empty
  kit bar for now), `ModeChanged` → enter or leave, the right panel state
  after every update (old `finish_update`), `SidebarIntent::RevealRight`,
  Docker events and logs in `subscription.rs`.
- 14 containers: the row menu, the confirm and create dialogs (layers,
  `top_overlay`, dialog check, `close_all_menus`).

**Clippy.** `crates/app/otty/clippy.toml` copies the root thresholds and the
root bans with reasons for the binary. `#![allow(clippy::disallowed_methods,
reason = ...)]` stands only at the top of the effects files that do I/O
(`startup`, `shell_integration`, `settings`, `modes`, the test-only
`temp_dir`), inside the test module of `effects/locale.rs` (it starts a copy
of the test program), and at the top of `subscription.rs`. A planted
`std::fs::read` in `update.rs` failed
`cargo clippy --workspace --all-targets --all-features -- -D warnings` with
`use of a disallowed method std::fs::read` and the note
`in otty-app, files are read and written in effects/`, so the nearer file is
read. The planted call was removed.

**Checks.** `cargo +nightly fmt --all -- --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo deny check` (only the 3
known yanked-crate warnings) and `cargo test --workspace --all-features`
pass; `cargo build -p otty-app` builds. The window was not opened here (no
display for the agent), so `cargo run -p otty-app` is left for the owner to
look at. `cargo llvm-cov` was not run, as asked.

**For the owner to decide.**

- `effects/settings.rs` repeats the reading rules of
  `SettingsData::from_json` for the language, the shell and the theme, so the
  frame has them before the settings feature exists. Issue 09 should replace
  it with the feature's own data rather than keep both.
- The settings and modes files are read synchronously in `App::new`, as the
  old app did, so the first frame already has the theme and the pins. Only
  the shell integration and the host name go through `Task::perform`.
- The old names stay where the meaning did not change (`AppEvent`,
  `Message::Input/Internal`, `ChromeIntent`, `MenuGuard`), so the two apps
  are easy to compare.
