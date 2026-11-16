//! The terminal of the OTTY app: the live terminal sessions of the tabs,
//! the panes they are split into and the context menu of a pane.
//!
//! The feature owns the live sessions (see the spec, "Exceptions"):
//! creating a terminal starts its shell through `otty-ui-term`, and the
//! sessions stay in the feature's state until their tab or pane closes.
//! How a terminal is started is given when the feature is created (see
//! [`StartTerminal`]), so a test can put a mock in place of the shell.
//! Everything else the feature needs from outside comes in as values: the
//! terminal settings with the session to start in
//! [`TerminalIntent::OpenTab`], the tab in front through
//! [`TerminalIntent::TabActivated`] and [`TerminalIntent::TabClosed`], and
//! what it reads at the moment of a decision in [`TerminalCtx`].
//!
//! It reports back with a [`TerminalOutcome`]: news for the tabs (a title,
//! a tab to close), the modes (a tab that started) and the explorer (the
//! folder of the active terminal), a failure to tell, and keyboard and
//! terminal operations the binary turns into iced tasks. Which pane takes
//! the keyboard after a tab closes or an overlay closes is decided here.
//!
//! # Example
//!
//! ```
//! use iced::{Point, Size};
//! use otty_app_slots::Component;
//! use otty_app_terminal::{TerminalCtx, TerminalIntent, TerminalWorkspace};
//!
//! let ctx = TerminalCtx {
//!     pane_grid_size: Size::new(800.0, 600.0),
//!     screen_size: Size::new(1000.0, 600.0),
//!     cursor: Point::ORIGIN,
//!     equalize_panes: true,
//!     is_terminal_mode: true,
//!     is_overlay_open: false,
//!     is_other_overlay_open: false,
//! };
//! let mut terminal = TerminalWorkspace::new();
//!
//! // The launch editor is in front: no terminal tab, so nothing to split.
//! let _ = terminal.reduce(TerminalIntent::TabActivated { tab_id: 3 }, ctx);
//! let split = terminal.reduce(
//!     TerminalIntent::SplitFocusedPane {
//!         axis: iced::widget::pane_grid::Axis::Vertical,
//!     },
//!     ctx,
//! );
//!
//! assert!(split.is_empty());
//! assert_eq!(terminal.focused_pane(), None);
//! ```

mod bindings;
mod block_duration;
mod blocks;
mod ctx;
mod errors;
mod event;
mod model;
mod pane_balance;
mod pane_fit;
mod reducer;
mod state;
#[cfg(test)]
mod testing;
mod types;
mod view;

use std::collections::HashMap;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

pub use blocks::BlockPlace;
pub use ctx::TerminalCtx;
pub use event::{TerminalIntent, TerminalOutcome};
use iced::Subscription;
use iced::widget::pane_grid;
use otty_app_slots::{Component, MainArea, MainContent, Overlay, Overlays};
use otty_app_theme::theme::ThemeProps;
use state::TerminalWorkspaceState;
pub use types::{PaneMachine, StartTerminal, TerminalEvent, TerminalKind};
use view::pane_context_menu::{self, PaneContextMenuProps};

use self::model::{BlockMenu, TerminalWorkspaceViewModel};

/// The screens the terminal shows in the main area. The panes of a
/// terminal tab scroll on their own, so the only screen is the line about
/// a terminal tab whose terminal is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScreen {
    /// The tab in front has no terminal.
    MissingTab,
}

/// The terminal feature: the terminal tabs with their panes and live
/// sessions, and the tab in front.
///
/// # Example
///
/// ```
/// use otty_app_terminal::TerminalWorkspace;
///
/// let terminal = TerminalWorkspace::new();
///
/// assert!(!terminal.has_context_menu());
/// assert_eq!(terminal.pane_count(), None);
/// ```
pub struct TerminalWorkspace {
    state: TerminalWorkspaceState,
    terminal_to_tab: HashMap<u64, u64>,
    next_terminal_id: u64,
    start_terminal: StartTerminal,
}

impl TerminalWorkspace {
    /// Return whether a pane menu is open. It is a menu over the window, so
    /// the frame treats it like the other menus.
    pub fn has_context_menu(&self) -> bool {
        self.state
            .tabs()
            .any(|(_, tab)| tab.context_menu().is_some())
    }

    /// Return the tab in front with its focused pane: the pane a split
    /// acts on. `None` when the tab in front is not a terminal tab.
    pub fn focused_pane(&self) -> Option<(u64, pane_grid::Pane)> {
        let tab_id = self.state.active_tab_id()?;
        let pane = self.state.tab(tab_id)?.focus()?;

        Some((tab_id, pane))
    }

    /// Return whether the tab in front is a terminal tab on this computer
    /// with a focused pane: the pane
    /// [`TerminalIntent::SplitFocusedPaneWith`] splits.
    pub fn has_local_pane_in_front(&self) -> bool {
        self.state
            .active_tab()
            .is_some_and(|tab| tab.runs_locally() && tab.focus().is_some())
    }

    /// Return the id of the terminal in the focused pane of the tab in
    /// front: the id the events of that terminal carry.
    pub fn focused_terminal_id(&self) -> Option<u64> {
        self.state.active_tab()?.focused_terminal_id()
    }

    /// Return the folder of the focused pane of the tab in front, shortened
    /// to its last folders when long: the one its shell reported, else the
    /// one the pane was started in; `None` when neither is known.
    pub fn focused_folder_path(&self) -> Option<&str> {
        let entry = self.state.active_tab()?.focused_terminal_entry()?;

        entry.label().folder_path()
    }

    /// Return whether a command runs in a pane of the tab in front. Its
    /// status counts the seconds, so the binary redraws the window once a
    /// second while this holds (see [`TerminalIntent::BlockClockTick`]).
    pub fn has_running_command_in_front(&self) -> bool {
        let Some(tab) = self.state.active_tab() else {
            return false;
        };

        tab.terminals().values().any(|entry| {
            let snapshot = entry.terminal().snapshot();
            blocks::has_running_command(snapshot.view().blocks())
        })
    }

    /// Return the place of the selected block among the command blocks of
    /// the focused pane of the tab in front, for the counter in the status
    /// bar. `None` with no block selected there: a block selected in
    /// another pane is not counted.
    pub fn selected_block_place(&self) -> Option<BlockPlace> {
        let tab = self.state.active_tab()?;
        let entry = tab.focused_terminal_entry()?;
        let block_key = tab.selected_block_key_in(entry.terminal().id)?;

        let snapshot = entry.terminal().snapshot();
        blocks::block_place(snapshot.view().blocks(), block_key)
    }

    /// Return the number of panes of the tab in front; `None` when it is
    /// not a terminal tab.
    pub fn pane_count(&self) -> Option<usize> {
        self.state.active_tab().map(|tab| tab.panes().len())
    }

    /// Create the feature with no terminal tab. The terminal of a new pane
    /// starts the program its settings name.
    pub fn new() -> Self {
        Self::starting_terminals_with(Rc::new(types::Terminal::new))
    }

    /// Create the feature with no terminal tab; `start_terminal` starts the
    /// terminal of every new pane.
    ///
    /// # Example
    ///
    /// ```
    /// use std::rc::Rc;
    ///
    /// use otty_app_terminal::TerminalWorkspace;
    ///
    /// let terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
    ///     otty_ui_term::Terminal::new,
    /// ));
    ///
    /// assert_eq!(terminal.pane_count(), None);
    /// ```
    pub fn starting_terminals_with(start_terminal: StartTerminal) -> Self {
        Self {
            state: TerminalWorkspaceState::default(),
            terminal_to_tab: HashMap::new(),
            next_terminal_id: 0,
            start_terminal,
        }
    }
}

impl Default for TerminalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TerminalWorkspace {
    type Intent = TerminalIntent;
    type Outcome = TerminalOutcome;
    type Ctx<'a> = TerminalCtx;

    fn reduce(
        &mut self,
        intent: TerminalIntent,
        ctx: TerminalCtx,
    ) -> Vec<TerminalOutcome> {
        reducer::reduce(
            &mut self.state,
            &mut self.terminal_to_tab,
            &mut self.next_terminal_id,
            &self.start_terminal,
            intent,
            &ctx,
        )
    }

    /// The events of every open terminal: its output, its title and its
    /// end. The sessions live here, so their events start here too.
    fn subscription(&self) -> Subscription<TerminalIntent> {
        let terminals = self.state.tabs().flat_map(|(_, tab)| {
            tab.terminals().values().map(|entry| {
                entry.terminal().subscription().map(TerminalIntent::Widget)
            })
        });

        Subscription::batch(terminals)
    }
}

impl MainArea for TerminalWorkspace {
    type Screen = TerminalScreen;

    /// The panes of the tab in front, each with its header. A tab whose
    /// terminal is missing shows a line about it. A running command shows
    /// its time up to the moment the view is built.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, TerminalIntent, TerminalScreen> {
        let vm =
            TerminalWorkspaceViewModel::from_state(&self.state, unix_time_ms());

        match vm.tab {
            Some(tab) => MainContent::ScrollsOnItsOwn(view::pane_grid::view(
                tab,
                theme.theme.ui(),
            )),
            None => MainContent::Screen {
                id: TerminalScreen::MissingTab,
                content: view::missing_tab(),
            },
        }
    }
}

impl Overlays for TerminalWorkspace {
    /// The open pane menu, at the pointer where it opened.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, TerminalIntent>> {
        let (&tab_id, tab, menu) =
            self.state.tabs().find_map(|(tab_id, tab)| {
                tab.context_menu().map(|menu| (tab_id, tab, menu))
            })?;

        let terminal_id = menu.terminal_id();
        // The block items act on the selected block of the menu's terminal.
        let block = tab.selected_block_key_in(terminal_id).and_then(|key| {
            let entry = tab.terminals().get(&terminal_id)?;
            let snapshot = entry.terminal().snapshot();

            Some(BlockMenu::of_block(&snapshot.view(), key))
        });
        let content = pane_context_menu::view(PaneContextMenuProps {
            tab_id,
            pane: menu.pane(),
            cursor: menu.cursor(),
            grid_size: menu.grid_size(),
            terminal_id: menu.terminal_id(),
            block,
            theme,
        });

        Some(Overlay::menu(content))
    }
}

/// The time now in milliseconds since 1970: the clock the terminal stamps
/// the start and the end of a command with. 0 when the clock is before
/// 1970.
fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use iced::advanced::widget::operation;
    use iced::event::Status;
    use iced::{Point, Size};
    use iced_test::runtime::user_interface::{Cache, UserInterface};
    use otty_app_domain::keys::{Platform, ShortcutKey};
    use otty_app_domain::shortcuts::AppShortcut;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::OverlayKind;
    use otty_app_testing::blocks::{
        command_block, command_history, finished, key_of, snapshot_in,
    };
    use otty_app_testing::terminal::ScriptedTerminals;
    use otty_app_theme::theme::ThemeManager;
    use otty_libterm::escape::NamedPrivateMode;
    use otty_libterm::surface::{
        BlockKey, SnapshotOwned, Surface, SurfaceActor, SurfaceConfig,
        SurfaceModel,
    };
    use otty_ui_kit::components::target_dot::TargetKind;
    use otty_ui_term::BlockCommand;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::*;

    #[cfg(unix)]
    const TEST_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const TEST_SHELL_PATH: &str = "cmd.exe";

    /// A workspace whose terminals run over mocks, so no shell starts: a
    /// real shell needs a pty and a process, which a busy machine may
    /// refuse.
    fn scripted_workspace() -> TerminalWorkspace {
        TerminalWorkspace::starting_terminals_with(Rc::new(
            ScriptedTerminals::default().starter(),
        ))
    }

    /// Open a terminal tab with one shell pane; it comes to the front.
    fn open_shell_tab(terminal: &mut TerminalWorkspace, tab_id: u64) {
        let mut settings = Settings::default();
        settings.engine = settings.engine.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(TEST_SHELL_PATH),
            ),
        );

        let _ = terminal.reduce(
            TerminalIntent::OpenTab {
                tab_id,
                title: String::from("sh"),
                settings: Box::new(settings),
                kind: TerminalKind::Shell,
                machine: PaneMachine {
                    name: String::from("build-box"),
                    target: TargetKind::Local,
                    home: None,
                },
            },
            ctx(Size::new(800.0, 600.0)),
        );
    }

    /// Tell the feature that terminal `terminal_id` has a new snapshot.
    fn content_sync(terminal: &mut TerminalWorkspace, terminal_id: u64) {
        let _ = terminal.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::ContentSync {
                id: terminal_id,
            }),
            ctx(Size::new(800.0, 600.0)),
        );
    }

    /// The feature starts the terminal of a new tab the way it was given
    /// at construction: here over a mock, whose snapshot the pane reads.
    #[test]
    fn a_new_tab_starts_its_terminal_the_way_the_feature_was_given() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let terminal_id = terminal.focused_terminal_id().expect("the pane");

        terminals.report(terminal_id, snapshot_in("/tmp/shown"));
        content_sync(&mut terminal, terminal_id);

        assert_eq!(terminal.focused_folder_path(), Some("/tmp/shown"));
    }

    /// A snapshot with one command block that `is_finished` or still runs.
    fn snapshot_with_command(is_finished: bool) -> SnapshotOwned {
        let mut block = command_block("cmd-1", 1_000);
        if is_finished {
            block = finished(block, Some(0), 1_000);
        }

        let mut snapshot = SnapshotOwned::default();
        snapshot.blocks.push(block);

        snapshot
    }

    /// The status of a running command counts the seconds, so the app
    /// redraws while one runs in a pane of the tab in front, and stops
    /// when it ends or another tab comes to the front.
    #[test]
    fn a_command_runs_in_front_only_while_a_pane_of_the_tab_in_front_runs_one()
    {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let terminal_id = terminal.focused_terminal_id().expect("the pane");
        let at_prompt = terminal.has_running_command_in_front();

        terminals.report(terminal_id, snapshot_with_command(false));
        content_sync(&mut terminal, terminal_id);
        let running = terminal.has_running_command_in_front();
        let _ = terminal.reduce(
            TerminalIntent::TabActivated { tab_id: 9 },
            ctx(Size::new(800.0, 600.0)),
        );
        let in_another_tab = terminal.has_running_command_in_front();
        let _ = terminal.reduce(
            TerminalIntent::TabActivated { tab_id: 1 },
            ctx(Size::new(800.0, 600.0)),
        );
        terminals.report(terminal_id, snapshot_with_command(true));
        content_sync(&mut terminal, terminal_id);

        assert!(!at_prompt);
        assert!(running);
        assert!(!in_another_tab);
        assert!(!terminal.has_running_command_in_front());
    }

    /// Tell the feature that the block `block_key` of terminal
    /// `terminal_id` was clicked.
    fn select_block(
        terminal: &mut TerminalWorkspace,
        terminal_id: u64,
        block_key: BlockKey,
    ) {
        let _ = terminal.reduce(
            TerminalIntent::Widget(otty_ui_term::Event::BlockSelected {
                id: terminal_id,
                block_key,
            }),
            ctx(Size::new(800.0, 600.0)),
        );
    }

    const SIX_COMMANDS: [&str; 6] =
        ["cmd-1", "cmd-2", "cmd-3", "cmd-4", "cmd-5", "cmd-6"];

    /// The status bar counts the selected block among the command blocks
    /// of the focused pane; the prompt is not counted.
    #[test]
    fn the_fifth_of_six_selected_command_blocks_is_counted_5_of_6() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let terminal_id = terminal.focused_terminal_id().expect("the pane");
        terminals.report(terminal_id, command_history(&SIX_COMMANDS));
        content_sync(&mut terminal, terminal_id);
        let unselected = terminal.selected_block_place();

        select_block(&mut terminal, terminal_id, key_of("cmd-5"));

        assert_eq!(unselected, None);
        assert_eq!(
            terminal.selected_block_place(),
            Some(BlockPlace::new(5, 6))
        );
    }

    /// A block selected in a pane that does not have the focus gives no
    /// counter.
    #[test]
    fn a_block_selected_in_a_pane_that_is_not_focused_is_not_counted() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let first_id = terminal.focused_terminal_id().expect("the pane");
        let _ = terminal.reduce(
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
            ctx(Size::new(800.0, 600.0)),
        );
        let split_id = terminal.focused_terminal_id().expect("the new pane");
        for terminal_id in [first_id, split_id] {
            terminals.report(terminal_id, command_history(&SIX_COMMANDS));
            content_sync(&mut terminal, terminal_id);
        }

        select_block(&mut terminal, first_id, key_of("cmd-5"));

        assert_ne!(terminal.focused_terminal_id(), Some(first_id));
        assert_eq!(terminal.selected_block_place(), None);
    }

    /// The second that passes changes nothing but the time the statuses
    /// show.
    #[test]
    fn a_block_clock_tick_reports_nothing() {
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 1);

        let outcomes = terminal.reduce(
            TerminalIntent::BlockClockTick,
            ctx(Size::new(800.0, 600.0)),
        );

        assert!(outcomes.is_empty());
    }

    /// A pane born from a split starts its terminal the same way.
    #[test]
    fn a_split_pane_starts_its_terminal_the_way_the_feature_was_given() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let first_id = terminal.focused_terminal_id().expect("the pane");
        let _ = terminal.reduce(
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
            ctx(Size::new(800.0, 600.0)),
        );
        let split_id = terminal.focused_terminal_id().expect("the new pane");
        assert_ne!(split_id, first_id);

        terminals.report(split_id, snapshot_in("/tmp/split"));
        content_sync(&mut terminal, split_id);

        assert_eq!(terminal.pane_count(), Some(2));
        assert_eq!(terminal.focused_folder_path(), Some("/tmp/split"));
    }

    /// Context of a pane grid of `grid` in Terminal mode.
    fn ctx(grid: Size) -> TerminalCtx {
        TerminalCtx {
            pane_grid_size: grid,
            screen_size: grid,
            cursor: Point::ORIGIN,
            equalize_panes: true,
            is_terminal_mode: true,
            is_overlay_open: false,
            is_other_overlay_open: false,
        }
    }

    fn headless_renderer() -> iced::Renderer {
        use iced_test::core::renderer::Headless;

        iced_test::runtime::futures::futures::executor::block_on(
            <iced::Renderer as Headless>::new(
                iced::Font::DEFAULT,
                iced::Pixels(16.0),
                None,
            ),
        )
        .expect("a headless renderer")
    }

    #[test]
    fn given_terminal_tab_when_focused_pane_asked_then_tab_and_pane_return() {
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 3);
        let focus = terminal
            .state
            .tab(3)
            .and_then(|tab| tab.focus())
            .expect("shell tab should have a focused pane");

        assert_eq!(terminal.focused_pane(), Some((3, focus)));
        assert_eq!(terminal.pane_count(), Some(1));
        assert_eq!(terminal.focused_folder_path(), None);
    }

    /// A local shell tab in front has a pane that can run a program of this
    /// computer; the launch editor in front has none.
    #[test]
    fn a_local_terminal_tab_in_front_has_a_pane_to_split_here() {
        let mut terminal = scripted_workspace();
        let before = terminal.has_local_pane_in_front();
        open_shell_tab(&mut terminal, 3);
        let shell = terminal.has_local_pane_in_front();

        let _ = terminal.reduce(
            TerminalIntent::TabActivated { tab_id: 9 },
            ctx(Size::new(800.0, 600.0)),
        );

        assert!(!before);
        assert!(shell);
        assert!(!terminal.has_local_pane_in_front());
    }

    /// The launch editor is in front: the terminal tab stays, but no pane
    /// is focused, counted or split.
    #[test]
    fn given_no_terminal_tab_in_front_when_focused_pane_asked_then_none_returns()
     {
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 3);

        let _ = terminal.reduce(
            TerminalIntent::TabActivated { tab_id: 9 },
            ctx(Size::new(800.0, 600.0)),
        );

        assert_eq!(terminal.focused_pane(), None);
        assert_eq!(terminal.pane_count(), None);
    }

    /// Owner: panes split in a wider window are never closed when it
    /// shrinks. They get narrower than the minimum but stay inside the main
    /// area, so none covers the left panel. Nine panes fit 1000 px. At
    /// 1024×768 with both side panels open the grid is 696 px, 694 px
    /// without the lines beside the panels. At the full minimum iced would
    /// keep the eight right panes at 100 px, 807 px in all, and push the
    /// first pane out of the grid on the left.
    #[test]
    fn panes_squeezed_by_a_smaller_window_stay_inside_the_main_area() {
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 3);
        let wide = ctx(Size::new(1000.0, 705.0));
        let _ = terminal.reduce(TerminalIntent::SyncPaneGridSize, wide);
        for _ in 0..8 {
            let _ = terminal.reduce(
                TerminalIntent::SplitFocusedPane {
                    axis: pane_grid::Axis::Vertical,
                },
                wide,
            );
        }

        let small = ctx(Size::new(696.0, 705.0));
        let _ = terminal.reduce(TerminalIntent::SyncPaneGridSize, small);

        let vm = TerminalWorkspaceViewModel::from_state(&terminal.state, 0);
        let tab = vm.tab.expect("the shell tab is active");
        let real_grid = Size::new(694.0, 705.0);
        let regions = tab.panes.layout().pane_regions(
            otty_ui_kit::size::LINE,
            tab.min_pane_size,
            real_grid,
        );
        assert_eq!(regions.len(), 9);
        for region in regions.values() {
            assert!(region.x >= 0.0, "{region:?}");
            assert!(region.x + region.width <= real_grid.width, "{region:?}");
        }
    }

    /// The panes of a terminal fill the height they get and scroll on their
    /// own; a tab in front without a terminal is a screen with a line about
    /// it.
    #[test]
    fn the_main_area_shows_the_panes_of_the_tab_in_front() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut terminal = scripted_workspace();
        let is_missing_screen = matches!(
            terminal.main_area(theme),
            MainContent::Screen {
                id: TerminalScreen::MissingTab,
                ..
            }
        );

        open_shell_tab(&mut terminal, 1);

        assert!(is_missing_screen);
        assert!(matches!(
            terminal.main_area(theme),
            MainContent::ScrollsOnItsOwn(_)
        ));
    }

    /// The pane headers name the machine the tab was opened on.
    #[test]
    fn every_pane_header_names_the_machine_of_its_tab() {
        let themes = ThemeManager::new();
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 1);

        let content = terminal
            .main_area(ThemeProps::new(themes.current()))
            .into_element(ThemeProps::new(themes.current()));
        let mut ui = iced_test::simulator(content);

        assert!(ui.find("build-box").is_ok());
        assert!(ui.find("sh").is_ok());
    }

    /// The open pane menu is a menu over the window, and closes on a press
    /// outside it.
    #[test]
    fn the_open_pane_menu_is_a_menu_overlay() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 1);
        let closed = terminal.overlay(theme).is_none();
        let (tab_id, pane) =
            terminal.focused_pane().expect("the shell tab has a pane");
        let terminal_id = terminal
            .state
            .tab(tab_id)
            .and_then(|tab| tab.focused_terminal_id())
            .expect("the pane has a terminal");

        let _ = terminal.reduce(
            TerminalIntent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            },
            ctx(Size::new(800.0, 600.0)),
        );

        let menu = terminal.overlay(theme).expect("the menu should be open");
        assert!(closed);
        assert!(terminal.has_context_menu());
        assert_eq!(menu.kind(), OverlayKind::Menu);
    }

    /// The pane menu acts on the selected block, so a block chosen with
    /// the keys works there too: its items carry the key of that block.
    #[test]
    fn the_pane_menu_acts_on_a_block_chosen_with_the_keys() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let terminal_id = terminal.focused_terminal_id().expect("the pane");
        terminals.report(terminal_id, command_history(&SIX_COMMANDS));
        content_sync(&mut terminal, terminal_id);
        let (tab_id, pane) = terminal.focused_pane().expect("the pane");
        let _ = terminal.reduce(
            TerminalIntent::SelectPreviousBlock,
            ctx(Size::new(800.0, 600.0)),
        );
        let _ = terminal.reduce(
            TerminalIntent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            },
            ctx(Size::new(800.0, 600.0)),
        );

        let menu = terminal.overlay(theme).expect("the menu should be open");
        let mut ui = iced_test::simulator(menu.into_content());
        let _ = ui
            .click(i18n::t(Key::CopyCommand))
            .expect("the menu offers Copy command");

        let pressed: Vec<TerminalIntent> = ui
            .into_messages()
            .filter(|intent| {
                !matches!(intent, TerminalIntent::PaneGridCursorMoved { .. })
            })
            .collect();
        assert!(
            matches!(
                pressed.as_slice(),
                [TerminalIntent::RunBlockCommand {
                    tab_id: 1,
                    terminal_id: on,
                    command: BlockCommand::CopyCommand(block_key),
                }] if *on == terminal_id && *block_key == key_of("cmd-6")
            ),
            "got {pressed:?}"
        );
    }

    /// The input the window gets when `key` is pressed while the focused
    /// pane of the tab in front has the keyboard: what the widgets sent, and
    /// whether a widget captured the key, so the app keyboard subscription
    /// never sees it.
    fn press_in_focused_pane(
        terminal: &TerminalWorkspace,
        key: iced::keyboard::Event,
    ) -> (Vec<TerminalIntent>, Status) {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let widget = terminal
            .state
            .active_tab()
            .and_then(|tab| tab.focused_terminal_entry())
            .map(|entry| entry.terminal().widget_id().clone())
            .expect("the tab in front has a focused pane");
        let content = terminal.main_area(theme).into_element(theme);
        let mut renderer = headless_renderer();
        let mut ui = UserInterface::build(
            content,
            Size::new(800.0, 600.0),
            Cache::default(),
            &mut renderer,
        );
        ui.operate(&renderer, &mut operation::focusable::focus(widget));

        let mut input = Vec::new();
        let (_, statuses) = ui.update(
            &[iced::Event::Keyboard(key)],
            iced::mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut input,
        );

        let status = statuses.first().copied().unwrap_or(Status::Ignored);
        (input, status)
    }

    /// The press of `shortcut`'s keys on the current platform.
    fn shortcut_press(shortcut: AppShortcut) -> iced::keyboard::Event {
        use iced::keyboard::key::{Code, Named, NativeCode, Physical};
        use iced::keyboard::{Key, Location, Modifiers};

        let keys = shortcut.keys_on(Platform::current());
        let (physical_key, key) = match keys.key() {
            ShortcutKey::ArrowUp => {
                (Physical::Code(Code::ArrowUp), Key::Named(Named::ArrowUp))
            },
            ShortcutKey::ArrowDown => (
                Physical::Code(Code::ArrowDown),
                Key::Named(Named::ArrowDown),
            ),
            ShortcutKey::Character(character) => (
                Physical::Unidentified(NativeCode::Unidentified),
                Key::Character(character.to_string().into()),
            ),
            ShortcutKey::Digit1 => {
                (Physical::Code(Code::Digit1), Key::Character("1".into()))
            },
            ShortcutKey::Digit2 => {
                (Physical::Code(Code::Digit2), Key::Character("2".into()))
            },
            ShortcutKey::Digit3 => {
                (Physical::Code(Code::Digit3), Key::Character("3".into()))
            },
            ShortcutKey::Digit4 => {
                (Physical::Code(Code::Digit4), Key::Character("4".into()))
            },
            ShortcutKey::Digit5 => {
                (Physical::Code(Code::Digit5), Key::Character("5".into()))
            },
        };
        let held = keys.modifiers();
        let mut modifiers = Modifiers::empty();
        if held.ctrl() {
            modifiers |= Modifiers::CTRL;
        }
        if held.alt() {
            modifiers |= Modifiers::ALT;
        }
        if held.logo() {
            modifiers |= Modifiers::LOGO;
        }
        if held.shift() {
            modifiers |= Modifiers::SHIFT;
        }

        iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }
    }

    /// A focused terminal reports each shortcut and keeps the key, so the
    /// app keyboard subscription does not run it a second time.
    #[test]
    fn every_shortcut_in_a_focused_terminal_is_reported_once_and_kept() {
        let mut terminal = scripted_workspace();
        open_shell_tab(&mut terminal, 1);

        for shortcut in AppShortcut::ALL {
            let (input, status) =
                press_in_focused_pane(&terminal, shortcut_press(shortcut));

            let reported: Vec<AppShortcut> = input
                .iter()
                .filter_map(|intent| match intent {
                    TerminalIntent::Widget(
                        otty_ui_term::Event::BindingActionDispatched {
                            action,
                            ..
                        },
                    ) => Some(*action),
                    _ => None,
                })
                .collect();
            assert_eq!(reported, vec![shortcut]);
            assert_eq!(status, Status::Captured, "{shortcut:?}");
        }
    }

    /// The widget events in `input` that report a shortcut.
    fn reported_shortcuts(input: &[TerminalIntent]) -> Vec<AppShortcut> {
        input
            .iter()
            .filter_map(|intent| match intent {
                TerminalIntent::Widget(
                    otty_ui_term::Event::BindingActionDispatched {
                        action, ..
                    },
                ) => Some(*action),
                _ => None,
            })
            .collect()
    }

    /// A full-screen program (`vim`, `tmux`) has no blocks: the block keys
    /// pressed in its pane reach the program, and the app sees no shortcut.
    #[test]
    fn the_block_keys_reach_a_full_screen_program() {
        let terminals = ScriptedTerminals::default();
        let mut terminal = TerminalWorkspace::starting_terminals_with(Rc::new(
            terminals.starter(),
        ));
        open_shell_tab(&mut terminal, 1);
        let terminal_id = terminal.focused_terminal_id().expect("the pane");
        let size = otty_libterm::TerminalSize {
            cell_width: 8,
            cell_height: 16,
            cols: 40,
            rows: 10,
        };
        let mut surface = Surface::new(SurfaceConfig::default(), &size);
        surface.set_private_mode(
            NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
        );
        terminals.report(terminal_id, surface.snapshot_owned());
        content_sync(&mut terminal, terminal_id);

        for shortcut in [AppShortcut::PreviousBlock, AppShortcut::NextBlock] {
            let (input, status) =
                press_in_focused_pane(&terminal, shortcut_press(shortcut));

            let is_written = input.iter().any(|intent| {
                matches!(
                    intent,
                    TerminalIntent::Widget(otty_ui_term::Event::Write { .. })
                )
            });
            assert!(reported_shortcuts(&input).is_empty(), "{shortcut:?}");
            assert!(is_written, "{shortcut:?} should reach the program");
            assert_eq!(status, Status::Captured, "{shortcut:?}");
        }
    }
}
