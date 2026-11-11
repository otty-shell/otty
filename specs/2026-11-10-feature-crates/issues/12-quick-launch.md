# 12 Quick launch

Status: needs-triage
Blocked by: 11

## What to do

Move `app/src/widgets/quick_launch` into `otty-app-quick-launch`, following
"Moving a feature" in the spec. It fills three places: the Launches section of
the left panel (under the Sessions section of tabs), the launch editor in the
main area, and overlays (the context menu, the inline edit and the SSH secret
prompt).

- The SSH check and building the terminal settings
  (`quick_launch/services.rs`) go to `effects/quick_launch.rs` in the binary.
- Reading and saving the launches (`storage.rs`) go there too. A failed save
  becomes a notification once per streak of failures.
- A prepared launch returns `LaunchReady(TabLaunch)`. `TabLaunch` moves to
  `otty-app-domain`, and the binary hands it to tabs.
- Which machine a launch runs on is decided in this feature's `reduce`, not in
  the binary.
- The SSH secret prompt is always the top layer.

The spec's SSH example is the main test of this issue: write it as a binary
test first.

## Done when

- Saved commands and SSH launches, including a password or passphrase prompt
  and a retry, open their tabs as in the old app.
- The launch editor creates and edits launches; broken launch files at start
  become a notification.
- `otty-app-quick-launch` runs no processes and reads no files itself.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the quick launch feature crate

**`otty-app-quick-launch`** (`crates/app/features/quick-launch`) is a copy
of `app/src/widgets/quick_launch` without `services.rs` and `storage.rs`,
which went to `effects/quick_launch.rs` in the binary; the old widget is
unchanged (`git diff --stat 7ea5355 -- app` is empty). It depends on `iced`
(with `svg`), `log`, `serde`, `thiserror`, `otty-app-domain`,
`otty-app-i18n`, `otty-app-slots`, `otty-app-theme`, `otty-ui-kit`,
`otty-ui-term` (the terminal settings a ready launch hands to the tabs, as
the owner allowed in issue 10) and `otty-ui-tree`; for tests on
`iced_test`, `otty-app-testing` and `serde_json`. Its public API:

- `QuickLaunches` with `new(QuickLaunchFile)`, `launches()` and
  `top_level_launches()` (`LaunchSummary` with `path`, `title`, `is_ssh`,
  `into_path`), `has_context_menu`, `has_secret_prompt`, `has_inline_edit`,
  `tick_timer() -> Option<Duration>` (while a launch is checked or the
  tree waits to be saved; the binary runs the timer, as for the toasts),
  `launches_menu(theme)` with `launches_menu_height()` (the Launches header
  menu: New launch, New folder) and `secret_prompt(theme)`;
- `QuickLaunchFile` (serde, read and written by the binary),
  `QuickLaunchSetupOutcome`, `SshSecretRequest`/`SshSecretKind`,
  `LaunchRequest`, `ContextMenuAction`, `QuickLaunchType`,
  `QuickLaunchScreen`;
- `impl Feature` with `Ctx = QuickLaunchCtx` (in `ctx.rs`):
  `cursor` (where a context menu opens), `is_panel_resizing` (old
  `sidebar_is_resizing`) and `screen_size` (the context menu keeps the size
  it opened in, so the view draws only from the feature's state);
- `impl LeftPanel` (the Launches section, the second implementation after
  tabs), `impl MainArea` (`Screen = QuickLaunchScreen`: the editor of the
  tab in front, or a line that it is missing), `impl Overlays` (the context
  menu).

```rust
pub enum QuickLaunchOutcome {
    OpenEditorTab { title, init: WizardTabInit }, // -> tabs OpenWizardTab
    PrepareLaunch(LaunchRequest),                 // -> effects::quick_launch::prepare
    LaunchReady { title, settings, exit_behavior, machine }, // -> tabs OpenCommandTab
    CloseTab { tab_id },                          // -> tabs CloseTab
    SaveLaunches(QuickLaunchFile),                // -> effects::quick_launch::save
    FocusInput(Id),                               // -> operation::focus
    TakeKeyboard,                                 // -> focus NOTHING
    OverlayClosed,                                // -> terminal FocusActive
    Failed(String),                               // -> notifications Push
}
```

The intents are the old ones, with two changes: `TabClosed` carries
`new_active_id`, and `TabActivated { tab_id }` is new. Quick launch keeps
the tab in front itself, as the terminal does, so `main_area` draws the
editor of that tab without a tab argument.

`QuickLaunchEffect`, the `QuickLaunchEvent` wrapper and every `Task` of the
reducer are gone. The copied reducer tests were rewritten to check the
outcomes and failed against a reducer that reported nothing (19 of 99),
then `reduce` was written. The decisions the old effects made are in
`reduce` now: the machine of a ready launch (`TabMachine::Ssh(host)` or
`ThisComputer`) and the title of a new editor tab. A failed check comes
back as text: the error type of the check (`LaunchError`) moved to the
binary with the check; the feature keeps the title and editor errors.
`LaunchRequest` hides the typed secret from `Debug`, as the prompt state
did.

**Overlays in two layers.** The context menu lies among the mode overlays
and the SSH prompt on top of every layer. `Overlays::overlay` gives the
context menu (a menu). The prompt is the feature's own method,
`secret_prompt(theme) -> Option<Overlay>` (a dialog), which `view.rs`
draws right over Settings. A second method on the trait, even with a
default, would be an extension point with one implementation, which
AGENTS.md does not allow; the toasts of notifications set this pattern.
The inline edit is no overlay: it is drawn inside the tree, and the frame
only checks it (`Overlay::InlineEdit`, whose `kind()` is `None`; `kind()`
now returns `Option<OverlayKind>`).

**No domain `TabLaunch`.** The tabs already take
`OpenCommandTab { title, settings, exit_behavior, machine }`, and the
settings are an `otty-ui-term` type that `otty-app-domain` (no iced) can
not hold. `LaunchReady` carries the same four fields and the binary hands
them on field by field, as the spec says for types the two sides do not
read the same way. `TabMachine` and `WizardTabInit` stay in the domain.

**How the binary wires it:**

- `AppEvent::QuickLaunch(QuickLaunchIntent)`, `App::quick_launch`;
  `App::with_saved(saved, pinned, launches)`; `App::new` reads the
  launches file at start (as the settings and the pinned modes), and a
  broken file becomes a start toast through `tell_start_problems`;
- `dispatch/quick_launch.rs`: the ctx from `sidebar.cursor()`,
  `sidebar.is_resizing()` and `screen_size`, one call per outcome (above);
  `PrepareLaunch` reads the base terminal settings
  (`terminal::base_settings`) for the effect instead of a `Ctx` field, so
  no settings are built on every pointer move;
- `effects/quick_launch.rs`: `load_at_start`, `save` (atomic write,
  answers `PersistCompleted`/`PersistFailed`), `prepare` (validation, the
  SSH probe through `otty-libterm`, the session with the shell integration
  for a plain zsh/bash, `terminal_settings::for_session`; answers
  `SetupCompleted`). `otty-libterm` moved from the binary's
  dev-dependencies to its dependencies (a workspace crate, nothing new in
  `Cargo.lock`); `with_shell_integration` became `pub(crate)`;
- tabs: `Activated` → terminal and quick launch `TabActivated`; `Closed` →
  terminal and quick launch `TabClosed`; `WizardTabOpened` → terminal
  `TabActivated` (kept) and `WizardInitializeCreate`/`Edit`;
- sidebar `PanelEdgeDragged` → `ResetInteractionState`; palette: the
  launches in `PaletteCtx` and `LaunchPicked` → `LaunchNode`; commands:
  `NewLaunch` → `HeaderCreateCommand`, `NewFolder` → `RevealLaunches` then
  `HeaderCreateFolder`;
- `screens/terminal.rs`: the Launches section under Sessions
  (`launches_event` maps the frame messages), the editor in the main area,
  the Launches header menu, the top-level launches in the New session
  menu, the context menu in the mode overlays under the pane menu;
  `view.rs`: the SSH prompt as layer 7, the grips off under the prompt and
  the context menu; `subscription.rs`: the quick launch tick;
- `guards.rs`/`update.rs`: `SecretPrompt` on top in `top_overlay`,
  `LaunchContextMenu` and `InlineEdit` in Terminal mode, Esc for each, the
  prompt's answers in `is_dialog_answer`, the Launches hovers in
  `passes_every_dialog`, the Launches arm of `context_menu_guard`, the
  launch choices in `is_header_menu_choice`, `inline_edit_guard` and the
  inline edit check in `filter_input`, the context menu in
  `close_all_menus`, terminal input passing under an inline edit.

**Checklist from 05–11.** Resolved:

- 05: the Launches section (`ToggleLaunches`, `is_launches_open`), its
  header menu, the second half of `NewFolder` after `RevealLaunches`,
  `NewLaunch`, `PanelEdgeDragged`, `SidebarState::{is_resizing, cursor}`,
  the inline edit check, the Launches context menu, the SSH prompt layer
  and dialog check.
- 07: a Files hover keeps an inline edit.
- 08: `PaletteCtx::launches`, `LaunchPicked` → `LaunchNode`, the prompt
  over the palette.
- 09: the prompt over Settings (`top_overlay`, its layer, its answers, the
  old tests that it lies over Settings and takes the press on the dimmed
  area), the Launches hovers under a dialog.
- 10: the second `LeftPanel` in place of `empty_place()`, the Launches
  header menu, the top-level launches in the New session menu, the launch
  choices in `is_header_menu_choice`, `WizardTabOpened` fills the editor,
  `Closed` tells quick launch, `LaunchReady` → `OpenCommandTab` with the
  feature's `TabMachine`, the editor in the main area (with the
  feature's own `QuickLaunchScreen` instead of `TerminalScreen::Tab`), New
  launch in the menu and on the start card (still `AppCommand::NewLaunch`,
  see below).
- 11: `WizardTabOpened` also fills the editor (the terminal arm stays),
  `Closed` tells quick launch, the prompt over a pane menu, the inline
  edit and the Launches menu in `terminal_overlay`, the old focus test
  under the SSH prompt, the editor scrolling tests
  (`the_start_card_and_a_form_do_not_share_their_scrolling`,
  `two_launch_editors_do_not_share_their_scrolling`,
  `a_launch_editor_built_again_keeps_its_scrolling`).

Marks removed: `SidebarIntent::ToggleLaunches`, `HeaderMenu::Launches`,
`SidebarState::is_resizing` (and `SidebarWidget::is_resizing` left the
test-only impl). No "issue 12" note is left in `crates/app/`.

Not brought, with the reason: the Containers parts of the old tests (the
prompt over a Containers dialog and row menu, a press on the dimmed area
of a Containers dialog, a click in the Containers views cancelling an
inline edit) wait for 13 and 14. The old editor test
`given_any_language_when_drawn_then_the_optional_marker_is_in_that_language`
switches the process language and runs in a copy of the test program,
which a feature crate may not start (`Command::new` is banned), so it is
in the binary's `effects/locale.rs` next to the other test that runs
alone.

**Left for 13/14:** the Containers overlays in `top_overlay`,
`close_all_menus` and the grip check; the prompt over a Containers dialog
and row menu (`the_secret_prompt_lies_over_an_open_menu` covers the
Terminal menus and Containers mode with no Containers overlay); the
Containers presses in `inline_edit_guard` cancel an edit already (the
default arm).

**Tests.** `an_ssh_launch_that_asks_for_a_password_opens_its_tab_after_it`
in `dispatch/quick_launch.rs` is the spec's SSH example, written first and
failing against an empty delivery: the click (step 1–2), the answer that a
password is needed (3: the prompt is on top and takes the keyboard), the
password (4), and the ready launch (5–8): a tab "laptop" on the machine
"192.168.1.20" with one pane, Terminal mode from Containers mode, the tick
stopped. The ready launch's settings start a local shell, so no server is
contacted. A second test covers a wrong password and the retry.
Copied and ported: the reducer (outcomes first), state, model, types and
editor-draft tests, every view test of the widget, the services and
storage tests (to `effects/quick_launch.rs`), the old effects tests
(machine choice moved into the reducer tests), and from the old binary the
quick launch parts of the overlay rules, Esc, the dialog check, the inline
edit check, the prompt over menus and Settings, the editor scrolling, the
Launches menu at the pointer and the focus under the prompt. New: the
feature getters and places, the tab in front, a killed launch, the context
menu size, the dispatch of each outcome, the palette launches, New
folder, a failed save told once per streak, the tick subscription.
`otty-app-quick-launch` has 182 tests and 4 doc tests, `otty-app` 608
(525 before). The whole workspace runs 5086 tests, all passing.

**Checks.** `cargo tree -p otty-app-quick-launch -e normal` has no
`notify` and no other feature; `tokio` comes only through `otty-ui-term`
(a direct dependency for the settings, and through `otty-app-theme`), no
direct `tokio`. A planted `std::process::Command::new` and a planted
`std::fs::write` in the crate failed clippy with `use of a disallowed
method` and the notes `a feature runs no process; the binary does it in
effects/` and `a feature touches no files; the binary does it in
effects/`; they were removed. `cargo +nightly fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings`,
`cargo deny check` (only the 3 known yanked-crate warnings), `cargo test
--workspace --all-features` and `cargo build -p otty-app` pass. `cargo
llvm-cov` was not run, as asked. The window was not opened (no display
for the agent).

**For the owner to decide.**

- No `TabLaunch` in the domain (see above); `LaunchReady` mirrors
  `OpenCommandTab`.
- The SSH prompt is `QuickLaunches::secret_prompt`, not a second method
  of `Overlays`.
- New launch in the New session menu and on the start card stays
  `AppCommand::NewLaunch` (from issue 10): under an open menu it closes
  the menus and runs, where the old `HeaderCreateCommand` was a header
  menu choice; the result is the same. The Launches menu sends the old
  intents.
- Parsing the launches file is `serde_json` in the binary, because
  `QuickLaunchFile` derives serde; settings keep their parse in the
  feature (`SettingsLoad::parse`).
- Three tabs arms (`Activated`, `Closed`, `WizardTabOpened`) now tell two
  features each, one call per feature in a `Task::batch`.
- The binary tests of a launch rely on runs being numbered 1, 2, 3 in a
  fresh app; the feature does not expose the id otherwise.
