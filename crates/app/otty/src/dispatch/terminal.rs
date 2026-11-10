//! Delivers the intents of the terminal, with what it reads at the moment
//! of a decision, and reads what it reports: news for the tabs, the modes
//! and the explorer, a failure to tell, and the keyboard and terminal
//! operations of the window.

use iced::Task;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::mode::Mode;
use otty_app_explorer::ExplorerIntent;
use otty_app_modes::ModesIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Component;
use otty_app_tabs::TabsIntent;
use otty_app_terminal::{
    PaneMachine, TerminalCtx, TerminalIntent, TerminalKind, TerminalOutcome,
};
use otty_app_theme::fonts::FontsConfig;
use otty_ui_term::TerminalView;
use otty_ui_term::settings::Settings;

use super::{explorer, modes, notifications, tabs};
use crate::app::App;
use crate::guards::{self, Overlay};
use crate::message::AppEvent;
use crate::{layout, terminal_settings};

/// Give a terminal intent to the feature, with the room its panes have,
/// the pointer, the saved Equalize panes setting, the mode and the
/// overlays that are open, and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: TerminalIntent) -> Task<AppEvent> {
    let ctx = terminal_ctx(app);

    let outcomes = app.terminal.reduce(intent, ctx);

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

/// Start the user's shell in the new terminal tab `tab_id`, with the
/// settings of the theme and the session built at start.
pub(crate) fn open_shell_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
) -> Task<AppEvent> {
    let settings = terminal_settings::for_session(
        &base_settings(app),
        app.shell_session.session().clone(),
    );

    open_tab(app, tab_id, title, settings, TerminalKind::Shell)
}

/// Start the command of the new command tab `tab_id` with the settings it
/// came with.
pub(crate) fn open_command_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
    settings: Settings,
    exit_behavior: ExitBehavior,
) -> Task<AppEvent> {
    let kind = TerminalKind::Command { exit_behavior };

    open_tab(app, tab_id, title, settings, kind)
}

/// The settings new terminals start with: the colors of the current theme
/// and the terminal font, before a session is set.
pub(crate) fn base_settings(app: &App) -> Settings {
    terminal_settings::base(
        app.theme_manager.current(),
        &FontsConfig::default(),
    )
}

/// Give the terminal the new tab, which is the active tab, named after the
/// machine the tabs say it runs on, with the home folder read at start.
fn open_tab(
    app: &mut App,
    tab_id: u64,
    title: String,
    settings: Settings,
    kind: TerminalKind,
) -> Task<AppEvent> {
    let machine = PaneMachine {
        name: app.tabs.active_machine_name().to_string(),
        target: app.tabs.active_machine_target(),
        home: app.home.clone(),
    };

    deliver(
        app,
        TerminalIntent::OpenTab {
            tab_id,
            title,
            settings: Box::new(settings),
            kind,
            machine,
        },
    )
}

/// What the terminal reads at the moment of a decision.
fn terminal_ctx(app: &App) -> TerminalCtx {
    let top = guards::top_overlay(app);

    TerminalCtx {
        pane_grid_size: layout::pane_grid_size(
            app.screen_size,
            app.sidebar.side_panels_ratio(),
        ),
        screen_size: app.screen_size,
        cursor: app.sidebar.cursor(),
        equalize_panes: app.settings.saved().equalize_panes(),
        is_terminal_mode: app.modes.active() == Mode::Terminal,
        is_overlay_open: top.is_some(),
        is_other_overlay_open: top
            .is_some_and(|overlay| overlay != Overlay::TerminalContextMenu),
    }
}

fn read(app: &mut App, outcome: TerminalOutcome) -> Task<AppEvent> {
    match outcome {
        // Tabs only show in Terminal mode. A tab tells the modes feature
        // once its terminal runs, so a tab that can not start leaves the
        // mode as it was; the modes feature decides whether it switches.
        TerminalOutcome::TabStarted => {
            modes::deliver(app, ModesIntent::TabShown)
        },
        TerminalOutcome::TitleChanged {
            tab_id,
            title,
            folder,
        } => tabs::deliver(
            app,
            TabsIntent::SetTitle {
                tab_id,
                title,
                folder,
            },
        ),
        TerminalOutcome::CloseTab { tab_id } => {
            tabs::deliver(app, TabsIntent::CloseTab { tab_id })
        },
        TerminalOutcome::FolderFollowed { cwd } => {
            explorer::deliver(app, ExplorerIntent::SyncRoot { cwd })
        },
        TerminalOutcome::FocusTerminal(id) => TerminalView::focus(id),
        TerminalOutcome::TakeKeyboard => super::take_keyboard(),
        TerminalOutcome::RunBlockCommand { terminal, command } => {
            TerminalView::command(terminal, command)
        },
        TerminalOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use iced::widget::pane_grid;
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_settings::{SettingsIntent, SettingsLoad, SettingsLoadStatus};

    use super::*;
    use crate::dispatch::{self, settings};
    use crate::testing::window::{
        app, open_palette, open_pane_menu, open_settings, open_shell_tab,
        test_settings,
    };
    use crate::view::view;

    /// Owner, Q4 of the old app: a split shares the room by the saved
    /// Equalize panes setting, not by a change in the open Settings dialog
    /// that is not saved yet.
    #[test]
    fn the_terminal_reads_the_saved_equalize_setting_not_an_unsaved_one() {
        let mut app = app();
        let mut saved = test_settings();
        saved.set_equalize_panes(true);
        let _ = settings::deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                saved,
                SettingsLoadStatus::Loaded,
            )),
        );
        open_settings(&mut app);
        let _ = settings::deliver(
            &mut app,
            SettingsIntent::EqualizePanesToggled(false),
        );

        assert!(terminal_ctx(&app).equalize_panes);
    }

    /// The pane grid is the main area without the shown side panels; the
    /// pane menu is kept inside the whole main area and opens at the
    /// pointer.
    #[test]
    fn the_pane_grid_is_the_main_area_without_the_side_panels() {
        let mut app = app();
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(Size::new(
                1000.0, 700.0,
            ))),
        );

        let ctx = terminal_ctx(&app);

        assert_eq!(ctx.screen_size, app.screen_size);
        assert_eq!(
            ctx.pane_grid_size,
            layout::pane_grid_size(
                app.screen_size,
                app.sidebar.side_panels_ratio()
            )
        );
        assert!(ctx.pane_grid_size.width < ctx.screen_size.width);
        assert_eq!(ctx.cursor, app.sidebar.cursor());
    }

    /// The pane menu alone is the terminal's own overlay: a pane that
    /// closes closes it too. Another overlay over it stays.
    #[test]
    fn the_terminal_tells_its_own_menu_from_the_other_overlays() {
        let mut app = app();
        let plain = terminal_ctx(&app);
        open_pane_menu(&mut app);
        let own_menu = terminal_ctx(&app);
        open_palette(&mut app);
        let other = terminal_ctx(&app);

        assert!(!plain.is_overlay_open);
        assert!(plain.is_terminal_mode);
        assert!(own_menu.is_overlay_open && !own_menu.is_other_overlay_open);
        assert!(other.is_overlay_open && other.is_other_overlay_open);
    }

    /// Owner: a split with no room gives a toast. The window here is
    /// narrower than two panes of the minimum width.
    #[test]
    fn a_split_with_no_room_tells_the_user() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);
        let _ = dispatch::dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(Size::new(
                200.0, 600.0,
            ))),
        );

        let _ = deliver(
            &mut app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        );

        assert_eq!(app.terminal.pane_count(), Some(1));
        assert_eq!(app.notifications.unseen_count(), 1);
        let mut ui = simulator(view(&app));
        assert!(ui.find(i18n::t(Key::NoRoomForPane)).is_ok());
    }

    /// A split that fits shows both panes and tells nothing.
    #[test]
    fn a_split_that_fits_shows_both_panes() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);

        let _ = deliver(
            &mut app,
            TerminalIntent::SplitFocusedPane {
                axis: pane_grid::Axis::Vertical,
            },
        );

        assert_eq!(app.terminal.pane_count(), Some(2));
        assert_eq!(app.notifications.unseen_count(), 0);
    }

    /// The pane menu opens over the main area with its items.
    #[test]
    fn the_open_pane_menu_shows_over_the_window() {
        let mut app = app();

        open_pane_menu(&mut app);

        let mut ui = simulator(view(&app));
        assert!(ui.find(i18n::t(Key::CopySelection)).is_ok());
        assert!(ui.find(i18n::t(Key::ClosePane)).is_ok());
    }

    /// The shell of the only pane ends: its tab closes in the same update.
    #[test]
    fn a_shell_that_ends_closes_its_tab() {
        let mut app = app();
        let _ = open_shell_tab(&mut app);
        let terminal_id =
            app.terminal.focused_terminal_id().expect("the terminal");

        let _ = deliver(
            &mut app,
            TerminalIntent::Widget(otty_ui_term::Event::Shutdown {
                id: terminal_id,
                exit_status: std::process::ExitStatus::default(),
            }),
        );

        assert_eq!(app.tabs.len(), 0);
        assert_eq!(app.terminal.pane_count(), None);
    }
}
