//! Delivery, part of the App layer: holds [`AppEvent`] and hands each event
//! to the one that handles it. `dispatch/x.rs` delivers the intents of
//! widget `x` to its reducer; the effects go to `effects/x.rs`.

use iced::Task;
#[cfg(not(target_os = "macos"))]
use iced::window;
use iced::window::Direction;

use crate::app::{App, terminal_settings};
use crate::commands::AppCommand;
use crate::effects;
use crate::layout::screen_size_from_window;
use crate::shell_session::ShellSession;
use crate::theme::BuiltinTheme;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::containers::event::ContainersEvent;
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::modes::event::ModesEvent;
use crate::widgets::notifications::event::NotificationsEvent;
use crate::widgets::palette::event::PaletteEvent;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::sidebar::event::SidebarEvent;
use crate::widgets::tabs::event::TabsEvent;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod containers;
pub(crate) mod explorer;
pub(crate) mod modes;
pub(crate) mod notifications;
pub(crate) mod palette;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

/// An event the app loop carries: a widget event, a command or an event of
/// the window itself.
#[derive(Clone)]
pub(crate) enum AppEvent {
    Sidebar(SidebarEvent),
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    QuickLaunch(QuickLaunchEvent),
    TerminalWorkspace(TerminalWorkspaceEvent),
    Explorer(ExplorerEvent),
    Settings(SettingsEvent),
    Palette(PaletteEvent),
    Modes(ModesEvent),
    Containers(ContainersEvent),
    Notifications(NotificationsEvent),
    /// Run an app command, from a shortcut, a palette entry or a header
    /// button.
    Command(AppCommand),
    /// The window is ready; the startup effects run.
    IcedReady,
    /// The effects built the session new terminal tabs start, after they
    /// wrote the shell integration files. It has no integration when the
    /// files could not be written or the shell has none.
    ShellIntegrated(ShellSession),
    /// The effects read this computer's host name; `None` when it could not
    /// be read.
    HostNameRead(Option<String>),
    /// Saved settings chose this theme for the window and new terminals.
    ThemeChosen(BuiltinTheme),
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

/// Hand an app event to the one that handles it: a widget intent to the
/// widget's delivery in `dispatch/`, a widget effect to the widget's effects
/// file in `effects/`, a command to [`command::deliver`]. The events of the
/// window itself are handled here.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => effects::startup::run(app),
        AppEvent::ShellIntegrated(shell_session) => {
            app.shell_session = shell_session;

            Task::none()
        },
        AppEvent::HostNameRead(host_name) => {
            app.host_name = host_name;

            Task::none()
        },
        AppEvent::ThemeChosen(theme) => {
            app.theme_manager.set_theme(theme);
            app.terminal_settings =
                terminal_settings(app.theme_manager.current(), &app.fonts);

            Task::none()
        },
        AppEvent::Sidebar(SidebarEvent::Intent(intent)) => {
            sidebar::deliver(app, intent)
        },
        AppEvent::Sidebar(SidebarEvent::Effect(effect)) => {
            effects::sidebar::run(effect)
        },
        AppEvent::Chrome(ChromeEvent::Intent(intent)) => {
            chrome::deliver(app, intent)
        },
        AppEvent::Chrome(ChromeEvent::Effect(effect)) => {
            effects::chrome::run(effect)
        },
        AppEvent::Tabs(TabsEvent::Intent(intent)) => tabs::deliver(app, intent),
        AppEvent::Tabs(TabsEvent::Effect(effect)) => {
            effects::tabs::run(app, effect)
        },
        AppEvent::QuickLaunch(QuickLaunchEvent::Intent(intent)) => {
            quick_launch::deliver(app, intent)
        },
        AppEvent::QuickLaunch(QuickLaunchEvent::Effect(effect)) => {
            effects::quick_launch::run(effect)
        },
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(intent)) => {
            terminal_workspace::deliver(app, intent)
        },
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Effect(effect)) => {
            effects::terminal_workspace::run(app, effect)
        },
        AppEvent::Explorer(ExplorerEvent::Intent(intent)) => {
            explorer::deliver(app, intent)
        },
        AppEvent::Explorer(ExplorerEvent::Effect(effect)) => {
            effects::explorer::run(app, effect)
        },
        AppEvent::Settings(SettingsEvent::Intent(intent)) => {
            settings::deliver(app, intent)
        },
        AppEvent::Settings(SettingsEvent::Effect(effect)) => {
            effects::settings::run(effect)
        },
        AppEvent::Palette(PaletteEvent::Intent(intent)) => {
            palette::deliver(app, intent)
        },
        AppEvent::Palette(PaletteEvent::Effect(effect)) => {
            effects::palette::run(effect)
        },
        AppEvent::Modes(ModesEvent::Intent(intent)) => {
            modes::deliver(app, intent)
        },
        AppEvent::Modes(ModesEvent::Effect(effect)) => {
            effects::modes::run(effect)
        },
        AppEvent::Containers(ContainersEvent::Intent(intent)) => {
            containers::deliver(app, intent)
        },
        AppEvent::Containers(ContainersEvent::Effect(effect)) => {
            effects::containers::run(app, effect)
        },
        AppEvent::Notifications(NotificationsEvent::Intent(intent)) => {
            notifications::deliver(app, intent)
        },
        AppEvent::Notifications(NotificationsEvent::Effect(effect)) => {
            effects::notifications::run(effect)
        },
        AppEvent::Command(command) => command::deliver(command),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state.set_screen_size(screen_size_from_window(size));

            terminal_workspace::deliver(
                app,
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )
        },
        #[cfg(target_os = "macos")]
        AppEvent::ResizeWindow(_) => Task::none(),
        #[cfg(not(target_os = "macos"))]
        AppEvent::ResizeWindow(dir) => {
            window::latest().and_then(move |id| window::drag_resize(id, dir))
        },
        // A shortcut key arrives as its command (see `update_input`); other
        // keys no widget took do nothing.
        AppEvent::Keyboard(_) | AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::pane_grid;
    use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

    use super::*;
    use crate::services;
    use crate::shell_integration::{
        integrated_shell_session, shell_integration_dir,
    };
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::settings::event::SettingsEffect;
    use crate::widgets::settings::types::SettingsData;
    use crate::widgets::sidebar::event::SidebarIntent;
    use crate::widgets::tabs::event::TabsIntent;

    /// A shell without OTTY shell integration, so building its session
    /// writes no files.
    const TEST_SHELL_PATH: &str = "/opt/otty-test/otty-test-shell";

    #[test]
    fn a_read_host_name_is_written_to_the_app() {
        let (mut app, _) = App::new();

        let _none = dispatch(
            &mut app,
            AppEvent::HostNameRead(Some(String::from("build-box"))),
        );

        assert_eq!(app.host_name.as_deref(), Some("build-box"));
    }

    #[test]
    fn an_integrated_shell_session_is_written_to_the_app() {
        let (mut app, _) = App::new();
        let session = ShellSession::without_integration(TEST_SHELL_PATH);

        let _none = dispatch(&mut app, AppEvent::ShellIntegrated(session));

        assert_eq!(app.shell_session.name(), "otty-test-shell");
        assert_eq!(local_options(&app).program(), TEST_SHELL_PATH);
    }

    #[test]
    fn a_chosen_theme_colors_the_window_and_new_terminals() {
        let (mut app, _) = App::new();

        for theme in BuiltinTheme::ALL {
            let _none = dispatch(&mut app, AppEvent::ThemeChosen(theme));

            assert_eq!(app.theme_manager.current().ui(), &theme.ui_palette());
            assert_eq!(
                app.terminal_settings.theme.color_pallete.background,
                theme.terminal_palette().background,
            );
        }
    }

    /// The first terminal starts with the shell session in the app, so the
    /// startup effects must put the integrated session there first.
    #[test]
    fn the_first_tab_opens_once_the_app_has_the_shell_session_and_host_name() {
        let (mut app, _) = App::new();
        let shell_path = app.widgets.settings.saved().terminal_shell();
        let integrated =
            integrated_shell_session(shell_path, &shell_integration_dir());
        let SessionKind::Local(expected) = integrated.session() else {
            panic!("the shell session should be local");
        };

        assert_eq!(app.host_name, None);

        let events = collect_outputs(dispatch(&mut app, AppEvent::IcedReady));
        let first_tab = deliver_until(&mut app, events, |event| {
            matches!(
                event,
                AppEvent::Tabs(TabsEvent::Intent(TabsIntent::OpenTerminalTab))
            )
        });

        assert!(first_tab.is_some());
        assert_eq!(app.host_name, services::read_host_name());
        assert_eq!(local_options(&app).args(), expected.args());
        assert_eq!(local_options(&app).envs(), expected.envs());
    }

    /// New terminals use the app's terminal settings and shell session, so
    /// saved settings reach the app before the open terminals are recolored.
    #[test]
    fn applied_settings_reach_the_app_before_the_terminals() {
        let (mut app, _) = App::new();
        app.theme_manager.set_theme(BuiltinTheme::OttyDark);
        let mut saved = SettingsData::default();
        saved.set_theme(BuiltinTheme::OttyLight);
        saved.set_terminal_shell(String::from(TEST_SHELL_PATH));

        let events = collect_outputs(dispatch(
            &mut app,
            AppEvent::Settings(SettingsEvent::Effect(
                SettingsEffect::ApplyTheme(saved),
            )),
        ));
        let recolor = deliver_until(&mut app, events, |event| {
            matches!(
                event,
                AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::ApplyTheme { .. }
                ))
            )
        });

        assert!(recolor.is_some());
        assert_eq!(
            app.theme_manager.current().ui(),
            &BuiltinTheme::OttyLight.ui_palette(),
        );
        assert_eq!(
            app.terminal_settings.theme.color_pallete.background,
            BuiltinTheme::OttyLight.terminal_palette().background,
        );
        assert_eq!(app.shell_session.name(), "otty-test-shell");
    }

    #[test]
    fn smaller_window_keeps_the_share_panels_come_back_at() {
        let (mut app, _) = App::new();
        let pane_grid::Node::Split { id: split, .. } =
            app.widgets.sidebar.panes().layout()
        else {
            panic!("sidebar should split the left panel from the rest");
        };
        let resize = pane_grid::ResizeEvent {
            split: *split,
            ratio: 0.3,
        };
        let _wider = app.widgets.sidebar.reduce(SidebarIntent::Resized(resize));
        let _close = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);

        let smaller = iced::Size::new(400.0, 300.0);
        let _resized = dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _open = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);

        let _enter = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        let _resized = dispatch(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _leave = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);
    }

    /// Deliver `events` in order, as the app loop does, until one matches
    /// `is_stop`; return that event without delivering it.
    fn deliver_until(
        app: &mut App,
        events: Vec<AppEvent>,
        is_stop: fn(&AppEvent) -> bool,
    ) -> Option<AppEvent> {
        for event in events {
            if is_stop(&event) {
                return Some(event);
            }

            let _next = dispatch(app, event);
        }

        None
    }

    /// The launch options of the app's shell session.
    fn local_options(app: &App) -> &LocalSessionOptions {
        let SessionKind::Local(options) = app.shell_session.session() else {
            panic!("the shell session should be local");
        };

        options
    }
}
