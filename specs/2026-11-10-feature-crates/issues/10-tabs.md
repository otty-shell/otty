# 10 Tabs

Status: needs-triage
Blocked by: 09

## What to do

Move `app/src/widgets/tabs` into `otty-app-tabs`, following "Moving a feature"
in the spec. It adds the `LeftPanel` trait to `otty-app-slots`.

- The Sessions section draws only from the tabs state. The host name arrives
  as an intent (`HostNameRead`) instead of being passed to the view.
- `TabMachine` and `WizardTabInit` move to `otty-app-domain`, since tabs and
  quick launch both read them.
- Opening, activating and closing a tab return outcomes that carry what other
  features need, for example `Closed { tab_id, new_active_id }`. The decisions
  about what the terminal does then belong to the terminal feature (issue 11),
  not to the binary.
- Tabs are only terminal sessions and the launch editor; settings is not a tab.

## Done when

- The Sessions section, the new session menu and the tab list grouped by machine
  work as in the old app, except what needs the terminal.
- `otty-app-tabs` depends on no other feature.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the tabs feature crate

**`otty-app-tabs`** (`crates/app/features/tabs`) is a copy of
`app/src/widgets/tabs` without `view/new_session_menu.rs` and
`view/empty_state.rs`, which went to the binary (see below); the old widget
is unchanged (`git diff --stat 0e34720 -- app` is empty). It depends on
`iced` (with `svg`), `otty-app-domain`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme`, `otty-ui-kit` and `otty-ui-term` (for the terminal
settings a command tab hands on); for tests on `iced_test` and
`otty-app-testing`. Its public API:

- `Tabs` with `new`, `active_tab_id`, `active_tab_content`,
  `active_tab_title`, `len`, `is_empty`, `active_machine_name` and
  `active_machine_target` (this computer while no tab is open; the status
  bar reads them) and `sessions() -> Vec<OpenSession>` (the open tabs in
  the grouping of the Sessions section; the palette ctx is built from it);
- `TabContent` (`Terminal`, `QuickLaunchWizard`), `OpenSession` (getters
  only) and `SESSION_LIST_SCROLL_ID`;
- `impl Feature` with `Ctx<'a> = TabsCtx<'a> { pub shell_name: &'a str }`
  in `ctx.rs` (the old `TabsCtx`);
- `impl LeftPanel`: the Sessions section. No `Overlays` (see below).

```rust
pub enum TabsIntent {
    ActivateTab { tab_id }, CloseTab { tab_id }, CloseActive,
    SetTitle { tab_id, title, folder }, OpenTerminalTab,
    OpenCommandTab { title, settings, exit_behavior, machine },
    OpenWizardTab { title, init }, HostNameRead(Option<String>),
}

pub enum TabsOutcome {
    /// A tab came to the front without waiting for its terminal (activated,
    /// or the launch editor opened): another mode switches to Terminal.
    Shown,
    Activated { tab_id },
    Closed { tab_id, new_active_id },
    TerminalTabOpened { tab_id, title },
    CommandTabOpened { tab_id, title, settings, exit_behavior },
    WizardTabOpened { tab_id, init },
    /// A tab was added at the end: scroll the Sessions list to its end.
    ScrollSessionsToEnd,
}
```

`TabsEffect`, the `TabsEvent` wrapper and the `Task::done` of the reducer
are gone. The copied reducer tests were rewritten to check the returned
outcomes and run first against the copied `Task` reducer (they failed to
build: `expected Vec<TabsOutcome>, found Task<TabsEvent>`), then `reduce`
changed. The decision the old `effects/tabs.rs` made (an activated tab and
the launch editor tell the modes widget at once, a terminal or command tab
waits for its terminal) is now in `reduce`: activating returns
`[Activated, Shown]`, the editor `[WizardTabOpened, ScrollSessionsToEnd,
Shown]`, a terminal tab `[TerminalTabOpened, ScrollSessionsToEnd]`.
`Closed` lost `remaining`: it was `> 0` exactly when `new_active_id` is
`Some`. The host name is no longer a view argument: it arrives as
`HostNameRead` and stays in the tabs state; `App::host_name` is gone.

**The `LeftPanel` trait** in `otty-app-slots` (`src/left_panel.rs`):

```rust
pub trait LeftPanel: Feature {
    fn left_panel<'a>(&'a self, theme: ThemeProps<'a>, section: SectionState)
        -> LeftPanelSection<'a, Self::Intent>;
}
pub struct SectionState { is_open, is_menu_open }   // new, is_open(), is_menu_open()
pub enum SectionMessage<Intent> { Toggle, OpenMenu, Feature(Intent) }
pub struct LeftPanelSection<'a, Intent> { content, content_height }
```

Why not the spec's `Element<'a, Self::Intent>`: in Terminal mode the left
panel is a stack of sections (Sessions now, Launches with issue 12). The
frame keeps, as in the old app, which sections are open and which header
menu is open (the sidebar: `ToggleSessions`, `ToggleLaunches`,
`HeaderMenuOpen`), and it shares the panel height between the sections, so
it needs each section's content height. The section header is drawn by the
feature, but its title and its chevron are frame presses: the old view was
generic over exactly three messages (`on_toggle`, `on_menu`, `on_intent`),
which are now `SectionMessage`. The frame maps `Toggle` to
`SidebarIntent::ToggleSessions`, `OpenMenu` to
`HeaderMenuOpen(HeaderMenu::Sessions)` and `Feature` to `AppEvent::Tabs`
(`screens/terminal.rs`, `sessions_event`). The open/closed state comes in
with `SectionState`, like the theme; everything else the section draws is
the tabs' own state. The stacking stays in the sidebar: the old
`widgets/sidebar/view/left_panel.rs` is copied to
`sidebar/view/left_panel.rs` with its test. Quick launch (12) is the second
implementation. The containers engine tree (13) fills the whole panel and
has no frame parts; issue 13 decides whether it implements this trait with
`SectionMessage::Feature` only or gets its own method.

**The New session menu and the start card are frame views, not tabs
overlays.** The menu is a header menu of the sidebar: the sidebar keeps it
open or closed, where it opened and one header menu at a time for both
sections, and the frame closes it on Esc, a press outside and a shortcut.
Three of its four items belong to other features (Split to the terminal,
the launches and New launch to quick launch). So it is composed by the
frame, as the old `screens/terminal.rs` did: the view is copied to
`sidebar/view/new_session_menu.rs` (with its tests) and
`screens::terminal::header_menu` places it with its anchor
(`otty_ui_kit::geometry::anchor_position`, inside `App::screen_size`) and
its dismiss layer. The start card has a tabs tile and a quick launch tile;
it is copied to `screens/start_card.rs` (with its tests, its drawing test
uses `otty_app_testing::temp_dir`) and shown by `screens::terminal::main`
while no tab is open, as `MainContent::Screen` with the binary's
`TerminalScreen::StartCard`. Tabs therefore fills only the left panel.

**Moved to `otty-app-domain`:** `tab::TabMachine` (`ThisComputer`,
`Ssh(String)`, a plain enum) and `quick_launch::WizardTabInit` (`Create {
parent_path }`, `Edit { path, command }`). Naming a machine (it needs the
"This computer" text) and its target dot (an `otty-ui-kit` type) stay in
the tabs crate as `machine_name` and `machine_target`, with the old tests.

**How the binary wires it:**

- `AppEvent::Tabs(TabsIntent)` and `App::tabs`; `App::screen_size` (the
  main area, from `layout::screen_size_from_window`, copied with its tests)
  follows `Window(Resized)`, so the `expect(dead_code)` on
  `AppEvent::Window` is gone;
- `dispatch/tabs.rs`: builds `TabsCtx` from `app.shell_session.name()`,
  calls `reduce` and reads each outcome with one call: `Shown` →
  `modes::deliver(TabShown)`, `ScrollSessionsToEnd` →
  `operation::snap_to_end(SESSION_LIST_SCROLL_ID)`; `Activated`, `Closed`,
  `TerminalTabOpened`, `CommandTabOpened` and `WizardTabOpened` do nothing
  yet (notes on the arms name what 11 and 12 do there);
- `dispatch/mod.rs`: `HostNameRead` → `TabsIntent::HostNameRead`;
- `effects/startup.rs`: the first tab (`OpenTerminalTab`) is chained after
  the shell integration and the host name, as in the old app;
- `dispatch/command.rs`: `NewTerminal` → `OpenTerminalTab`, `CloseSession`
  → `CloseActive`;
- `dispatch/palette.rs`: `palette_ctx(&app.tabs, &app.modes, commands)`
  lists the sessions from `tabs.sessions()` (built from the `App` fields
  beside `app.palette`); `SessionPicked` → `ActivateTab`;
- `guards.rs`: `command_ctx` reads `has_active_tab`; `AppEvent::Tabs(_)`
  under a menu is `DismissAndRun` (old rule); `is_header_menu_choice`
  (New terminal for now); a dialog ignores tabs presses (added to
  `a_dialog_ignores_shortcuts_app_buttons_and_keys`);
- `update.rs`: a header menu choice closes the header menu and runs;
- `view.rs`: the title of the active tab in the header, the header menu
  layer right under the modes menu (layer 3), the left panel of Terminal
  mode; `screens/terminal.rs`: the Sessions section over an empty Launches
  place, the start card, the status bar (machine of the active tab, session
  count);
- `testing/task_outputs.rs` got `snaps_to_end(task, id)`,
  `testing/window.rs` got `open_terminal_tab`.

**Gaps resolved** (marks removed): `SidebarIntent::ToggleSessions` and
`HeaderMenuOpen`, `SidebarViewModel::{is_sessions_open, is_launches_open,
open_header_menu}` (the stacker reads the Launches state too),
`HeaderMenu` (the mark moved to its `Launches` variant, for 12),
`OpenHeaderMenu::{menu, cursor}`, `ModesIntent::TabShown`,
`ShellSession::name`, `AppEvent::Window`; without marks: the header menu
layer, its anchor and `screen_size_from_window`, the header menu choices
in `filter_input`, the first tab after the startup work, the title in the
header, `has_active_tab`, `NewTerminal`/`CloseSession`, the machine and
session count in the status bar, `PaletteCtx::sessions` (with the old test
`the_palette_lists_sessions_by_machine...`, sessions part) and
`SessionPicked`. No "issue 10" note is left in `crates/app/`.

**Left for issue 11 (terminal):**

- `TerminalTabOpened { tab_id, title }`: start the shell with
  `app.shell_session.session()` and the terminal settings, then send
  `ModesIntent::TabShown` once it started (a terminal tab does not switch
  the mode before). `CommandTabOpened { tab_id, title, settings,
  exit_behavior }`: the same with the command's settings.
- `Activated { tab_id }`: focus the active pane, sync the block selection,
  and send the folder of the tab's local terminal to the explorer
  (`SyncRoot`).
- `Closed { tab_id, new_active_id }`: end the tab's terminals; with
  `Some(new_active_id)` focus its pane, sync its selection and send its
  folder to the explorer. With `None` (no tab left) nothing gets focus.
- `TabsIntent::SetTitle` and `CloseTab` from the terminal (the focused
  pane's title and folder, a command that ended with `ExitBehavior::Close`).
- Opening a pressed file (`ExplorerOutcome::OpenFile`, note on its arm):
  needs the base terminal settings, so it was not wired here; it sends
  `TabsIntent::OpenCommandTab` with the editor's settings
  (`services::editor_terminal_settings`, which needs `shell_words`, already
  in `Cargo.lock`).
- The main area for an active tab (now empty) and the machine name in the
  pane headers (`app.tabs.active_machine_name()`); the Split item of the
  New session menu (`on_split`, now `None`) and the split in
  `is_header_menu_choice`; the folder and pane count in the status bar; the
  old start card scrolling tests in the window
  (`dragging_a_panel_split_across_the_start_card_does_not_scroll_it`,
  `in_a_small_window_each_tile_of_the_start_card_can_be_pressed`, ...),
  which need the window frame helpers.
- The terminal may need the host name for its pane headers: if it does
  not read it from the tabs, the `HostNameRead` arm in `dispatch/mod.rs`
  tells it too.

**Left for issue 12 (quick launch):** the Launches section in place of
`empty_place()` in `screens::terminal::left_panel` (a second `LeftPanel`),
the Launches header menu (`header_menu` returns `None` for it now), the
top-level launches in the New session menu, the launch choices in
`is_header_menu_choice`; `WizardTabOpened { tab_id, init }` → fill the
editor (`WizardInitializeCreate`/`Edit`), `Closed` → forget the tab;
`LaunchReady` → `OpenCommandTab` with the `TabMachine` the feature picks;
the launch editor in the main area (`TerminalScreen::Tab(tab_id)`); New
launch in the menu and on the start card now sends
`AppEvent::Command(AppCommand::NewLaunch)`, which does nothing yet — the
old app sent `QuickLaunchIntent::HeaderCreateCommand` (under an open menu
the start card tile then only closed the menu; through the command it
closes the menus and runs).

**Tests.** Copied and ported: the reducer (outcomes first), model, types
and Sessions view tests of the widget, the New session menu and start card
view tests, the left panel stacker test; from the old binary the tabs
dispatch tests that need no terminal (mode switches from Containers mode,
the tab named after the shell), the startup order with the first tab, the
header menu at the pointer, a middle press outside a header menu and on a
Sessions row under it, the tab press under an open menu, the launch editor
from Containers mode, `screen_size_from_window`, the palette sessions. New:
the host name intent, the getters, the left panel section and its frame
messages, a closed section, the scroll to the end, the header menu choice,
Close session by shortcut, the title in the header, the header menu under
the modes menu, `snaps_to_end`. `otty-app-tabs` has 48 tests and 4 doc
tests, `otty-app` 461 tests (409 before), `otty-app-slots` 9 doc tests (7
before), `otty-app-domain` 41 tests and 7 doc tests (5 before). The whole
workspace runs 4625 tests, all passing.

**Checks.** `cargo tree -p otty-app-tabs -e normal` has no `notify` and no
other feature; `tokio` comes only through `otty-ui-term` (a direct
dependency for the terminal settings of `OpenCommandTab`, and through
`otty-app-theme`), no direct `tokio`. A planted `std::fs::read` in the
crate failed clippy with `use of a disallowed method std::fs::read` and
the note `a feature touches no files; the binary does it in effects/`; it
was removed. `cargo +nightly fmt --all -- --check`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, `cargo deny check`
(only the 3 known yanked-crate warnings), `cargo test --workspace
--all-features` and `cargo build -p otty-app` pass. `cargo llvm-cov` was
not run, as asked. The window was not opened (no display for the agent).

**For the owner to decide.**

- `LeftPanel` differs from the spec: it takes the frame's `SectionState`
  and returns `LeftPanelSection` (content with `SectionMessage`, and the
  content height). The spec's table says tabs fills "left panel, overlays";
  with the New session menu as a frame menu it fills only the left panel.
- The New session menu and the start card live in the binary
  (`sidebar/view/new_session_menu.rs`, `screens/start_card.rs`), because
  their items belong to several features and the menu's state to the
  sidebar.
- `OpenCommandTab` still carries the `otty-ui-term` terminal settings
  through the tabs (`CommandTabOpened`), as in the old app; this is why the
  crate depends on `otty-ui-term`. Issue 12 moves `TabLaunch` to
  `otty-app-domain`, which has no iced, so the settings may have to be
  built by the terminal side from a plain launch instead.
- Until issue 11, the first tab opens at start but runs nothing: the
  Sessions section lists it and the main area is empty.
