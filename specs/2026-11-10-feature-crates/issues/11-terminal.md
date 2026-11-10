# 11 Terminal

Status: needs-triage
Blocked by: 10

## What to do

Move `app/src/widgets/terminal_workspace` into `otty-app-terminal`, following
"Moving a feature" in the spec. It adds the `MainArea` trait to
`otty-app-slots`.

The terminal feature owns the live terminal sessions (see the spec,
"Exceptions"); creating one starts its shell.

Link it to the features already moved:

- tabs: opening a tab opens its terminal; `Closed { tab_id, new_active_id }`
  reaches the terminal, and the terminal decides on focus and selection;
- explorer: the folder of the active local terminal reaches the explorer as
  `SyncRoot`;
- modes: a tab whose terminal started sends `TabShown`;
- overlays: when the last overlay closes in Terminal mode, the active pane gets
  the keys back.

Behaviour to keep (from the old app):

- `SplitRight` and `SplitDown` (Linux Ctrl+Shift+D, macOS Cmd+Shift+D); a split
  that does not fit is refused with a notification only;
- the smallest pane is 100 px, for dragging and for the check; after the window
  shrinks, panes squeeze but stay inside the main area;
- a focused terminal takes the keys it handles, so a shortcut runs once;
- the pane context menu and closing a pane.

## Done when

- Opening shells in tabs, splits, focus, closing panes and tabs work as in the
  old app.
- `otty-app-terminal` depends on no other feature.
- The steps and checks in "Moving a feature" are done.

## Comments

### 2026-11-10: the terminal feature crate

**`otty-app-terminal`** (`crates/app/features/terminal`) is a copy of
`app/src/widgets/terminal_workspace` with its tests; the old widget is
unchanged (`git diff --stat bb42824 -- app` is empty). It depends on `iced`,
`log`, `thiserror`, `otty-app-domain`, `otty-app-i18n`, `otty-app-slots`,
`otty-app-theme`, `otty-ui-kit` and `otty-ui-term`; for tests on
`iced_test`, `otty-app-testing` and `otty-libterm` (to build the terminal
snapshots a shell reports). It owns the live sessions (spec, "Exceptions"):
`TerminalTabState::new` and a split start the shell through `otty-ui-term`,
and the feature's own `subscription()` gives the events of every open
terminal. Everything else comes in as values. Its public API:

- `TerminalWorkspace` with `new`, `has_context_menu`, `focused_pane`
  (tab and pane a split acts on), `focused_terminal_id`,
  `focused_folder_path` and `pane_count` (the status bar) — all for the tab
  in front;
- `TerminalKind`, `PaneMachine { name, target, home }` (the machine the pane
  headers name, a plain input with `pub` fields), `TerminalEvent`,
  `TerminalScreen`;
- `impl Feature` with `Ctx<'a> = TerminalCtx` (in `ctx.rs`), `impl MainArea`
  (the panes of the tab in front as `ScrollsOnItsOwn`, or the "terminal tab
  not initialized" screen), `impl Overlays` (the open pane menu as
  `Overlay::menu`).

```rust
pub struct TerminalCtx {
    pub pane_grid_size: Size, pub screen_size: Size, pub cursor: Point,
    pub equalize_panes: bool, pub is_terminal_mode: bool,
    pub is_overlay_open: bool, pub is_other_overlay_open: bool,
}

pub enum TerminalIntent {
    OpenTab { tab_id, title, settings, kind, machine },
    TabActivated { tab_id }, TabClosed { tab_id, new_active_id },
    Widget(TerminalEvent), PaneClicked, PaneResized, PaneGridCursorMoved,
    OpenContextMenu, CloseContextMenu, SplitPane, SplitFocusedPane,
    ClosePane, CopySelection, PasteIntoPrompt, CopySelectedBlockContent,
    CopySelectedBlockPrompt, CopySelectedBlockCommand,
    ApplyTheme { palette }, CloseAllContextMenus, FocusActive, ModeChanged,
    SyncPaneGridSize,
}

pub enum TerminalOutcome {
    TabStarted,                              // -> modes TabShown
    TitleChanged { tab_id, title, folder },  // -> tabs SetTitle
    CloseTab { tab_id },                     // -> tabs CloseTab
    FolderFollowed { cwd },                  // -> explorer SyncRoot
    FocusTerminal(Id),                       // -> TerminalView::focus
    TakeKeyboard,                            // -> focus NOTHING
    RunBlockCommand { terminal, command },   // -> TerminalView::command
    Failed(String),                          // -> notifications Push
}
```

`TerminalWorkspaceEffect`, the `TerminalWorkspaceEvent` wrapper, every
`Task::done` and every `Task` of the reducer are gone (the state still
returns its private `StateCommand`, which the reducer turns into outcomes).
The copied reducer tests were rewritten to check the returned outcomes and
first ran against a reducer that returned nothing (40 of 124 failed), then
`reduce` was written. The decisions the old effects made are in `reduce`
now: `TabActivated` and `TabClosed { new_active_id }` decide the focus, the
block selection and the folder of the tab in front; `FocusActive` gives the
keys only in Terminal mode and with no overlay open (spec, "Overlays");
`ModeChanged` resizes the panes and focuses only back in Terminal mode; the
folder is computed in `reduce` and sent only when the tab in front is local
and its shell reported one (the old `SyncExplorer` hop read it later). The
terminal key bindings are built from `AppShortcut::keys_on` in the feature
(`bindings.rs`, with tests), and a view test checks that a focused terminal
reports every shortcut once and captures the key.

**The tab in front is the feature's own state.** The view draws only from
it, and so do the decisions and getters: `OpenTab` and `TabActivated` set
it, `TabClosed` sets `new_active_id`. The binary sends `TabActivated` for a
launch editor tab too (`WizardTabOpened`), so no pane of a hidden terminal
tab is focused, counted or split while the editor is in front.

**The `MainArea` trait** in `otty-app-slots` (`src/main_area.rs`):

```rust
pub trait MainArea: Feature {
    type Screen: Clone + PartialEq + 'static;
    fn main_area<'a>(&'a self, theme: ThemeProps<'a>)
        -> MainContent<'a, Self::Intent, Self::Screen>;
}
```

It differs from the spec by the associated `Screen`: `MainContent` (issue
05) names screens with each feature's own type, so the trait has to say
which. The frame picks the feature by the tab in front (`TabContent`); the
feature draws from its own state. Quick launch (launch editor) and
containers (lists) are the next implementations.

**How the binary wires it:**

- `AppEvent::Terminal(TerminalIntent)`, `App::terminal`;
  `dispatch/terminal.rs` builds `TerminalCtx` (`layout::pane_grid_size` of
  the main area and `sidebar.side_panels_ratio()`, `sidebar.cursor()`, the
  saved equalize switch, the mode, `top_overlay`), reads each outcome with
  one call, and has `open_shell_tab` / `open_command_tab` (the settings and
  the machine of the new tab from `app.shell_session`, `app.tabs` and the
  theme);
- `terminal_settings.rs`: the base settings from the theme and
  `FontsConfig::terminal()`, the session, and the editor settings with
  `shell-words` (already in `Cargo.lock` through the old app; new
  dependency of the binary only), with the old tests;
- tabs: `TerminalTabOpened`/`CommandTabOpened` → open the tab;
  `Activated` → `TabActivated`; `Closed` → `TabClosed`; `WizardTabOpened` →
  `TabActivated`; the terminal sends `SetTitle`, `CloseTab` and, once the
  terminal runs, `ModesIntent::TabShown`;
- explorer: `FolderFollowed` → `SyncRoot`; `OpenFile` → a command tab with
  `app.settings.terminal_editor()` (old test
  `a_file_opens_in_the_saved_editor_not_an_unsaved_one` ported);
- focus: `HeaderMenuClosed`, `MenuClosed(_)`, notifications `ListClosed`,
  palette `KeyboardReleased`, settings `Closed` → `FocusActive`;
  `ModeChanged(_)` → `ModeChanged`; the terminal decides;
- resizing: `Window(Resized)` and `MainAreaResized` → `SyncPaneGridSize`;
- settings `Applied` → `ApplyTheme` with the new theme's colors;
- commands: `SplitRight`/`SplitDown` → `SplitFocusedPane`; the New session
  menu's Split item → `SplitPane` of `focused_pane()`;
- `guards.rs`: `Overlay::TerminalContextMenu` (a menu, in Terminal mode
  under the header menus, Esc → `CloseAllContextMenus`), the pane menu
  rules in `context_menu_guard`, terminal reports in `passes_every_dialog`,
  `is_terminal_input_kept_out`, `has_focused_pane`, Split in
  `is_header_menu_choice`; `update.rs`: typed terminal input under a menu or
  a dialog is dropped first, `as_command` turns a terminal binding into its
  command, `close_all_menus` closes the pane menus;
- `view.rs` and `screens/terminal.rs`: the panes in the main area, the pane
  menu in the mode overlays, the grips off under it, folder and pane count
  in the status bar; `subscription.rs`: the terminal's subscription.

**Checklist from 05–10.** Resolved: main area for an active tab; terminal
settings from the theme and `fonts.terminal()`; key bindings from
`keys_on`; the binding path in `as_command`; terminal input kept out under
an overlay or dialog and terminal reports passing a dialog; `MainAreaResized`,
`Window`, `pane_grid_size`, `side_panels_ratio`; focus on
`HeaderMenuClosed`, `MenuClosed(Terminal)`, `ModeChanged(Terminal)`,
`ListClosed`, `KeyboardReleased`, settings `Closed`; `has_focused_pane`;
splits, the Split item and `is_header_menu_choice`; the pane context menu;
pane and folder in the status bar; terminal subscriptions;
`TerminalTabOpened`/`CommandTabOpened` with `TabShown` once started;
`Activated`; `Closed { tab_id, new_active_id }`; `SetTitle`/`CloseTab` from
the terminal; the folder of the active local terminal → `SyncRoot`;
`OpenFile` with the saved editor; theme colors to open terminals on
`Applied`; the equalize-panes setting; the machine name in the pane headers;
the old scrolling tests of 09 and 10 that need the window frame
(`settings_opened_again_starts_at_its_top`,
`settings_built_again_keeps_its_scrolling`,
`dragging_a_panel_split_across_the_start_card_does_not_scroll_it`,
`dragging_a_bar_of_the_start_card_only_scrolls_it`,
`in_a_small_window_each_tile_of_the_start_card_can_be_pressed`,
`a_terminal_tab_stays_out_of_the_scrolling_of_a_screen`, with
`testing/frames.rs`); the palette offers both splits with a terminal tab and
hides them outside Terminal mode. Marks removed: `ShellSession::session`,
`SidebarState::cursor` (the pane menu opens at it); `side_panels_ratio` and
`cursor` left the test-only impl of the sidebar. No "issue 11" note is left
in `crates/app/`.

Left, with the reason: the host name does not reach the terminal through
`HostNameRead` — the machine name is taken from the tabs when a tab opens,
and the first tab opens after the host name is read. The scrolling tests
of the launch editor (`the_start_card_and_a_form_do_not_share_their_scrolling`
and the editor ones) need quick launch (12).

**Left for later issues.** 12: `WizardTabOpened` also fills the editor (the
arm now sends `TabActivated`), `Closed` also tells quick launch, the SSH
prompt over a pane menu (`is_other_overlay_open` already counts any other
overlay), the inline edit and the Launches menu in `terminal_overlay`, the
old focus test under the SSH prompt. 13/14: the Containers overlays in
`top_overlay`; `ModeChanged` also enters and leaves Containers mode.

**Tests.** `otty-app-terminal` has 124 tests and 2 doc tests; `otty-app`
525 (461 before); `otty-app-slots` 10 doc tests (9 before);
`otty-app-testing` 4 doc tests (3 before, `TempDir::remove_now` for the
split test). The binary tests now open real shells: the test shell is
`/bin/sh` (no integration files), as the old app's tests did. The whole
workspace runs 4817 tests, all passing.

**Checks.** `cargo tree -p otty-app-terminal -e normal` has no `notify` and
no other feature; `tokio` comes only through `otty-ui-term` (a direct
dependency for the sessions, and through `otty-app-theme`), no direct
`tokio`. A planted `std::process::Command::new` in the crate failed clippy
with `use of a disallowed method std::process::Command::new` and the note
`a feature runs no process; the binary does it in effects/`; it was
removed. `cargo +nightly fmt --all -- --check`, `cargo clippy --workspace
--all-targets --all-features -- -D warnings`, `cargo deny check` (only the 3
known yanked-crate warnings), `cargo test --workspace --all-features` and
`cargo build -p otty-app` pass. `cargo llvm-cov` was not run, as asked. The
window was not opened (no display for the agent).

**For the owner to decide.**

- `MainArea` has an associated `Screen` type, which the spec's sketch did
  not have.
- The tab in front is kept by the terminal itself, so the binary also sends
  `TabActivated` when the launch editor opens (`WizardTabOpened`); issue 12
  adds the editor fill to that arm.
- `FocusActive` now gives nothing outside Terminal mode (spec, "Overlays");
  the old app focused a pane widget that was not on screen. A pane that
  exits still focuses the pane left in the tab in front, as before.
- The pane label still reads `HOME` once (`model.rs`) to write `~`; it is
  no file I/O and the clippy bans allow it, but it is environment reading
  in a feature.
- `TerminalWorkspace::focused_terminal_id` is public only because the
  binary's tests make a terminal report a folder.

### 2026-11-10: the binary reads HOME

The pane label no longer reads `HOME`. The binary reads it once in
`App::new` into `App::home` and gives it with every new tab as
`PaneMachine::home`; the tab passes it to its panes, which write a folder
inside it as `~`. A tab opened without a home shows folders in full. The
feature clippy bans now include `std::env::{var, var_os, vars}` (a planted
`std::env::var_os("HOME")` in the terminal crate failed clippy with the
new reason; it was removed). The settings feature still reads `SHELL` for
its default shell; it is marked with `#[expect(clippy::disallowed_methods)]`
and left for the owner to move. (Moved the same day; see issue 09.)
