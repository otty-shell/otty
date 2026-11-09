use iced::Task;
#[cfg(not(target_os = "macos"))]
use iced::window;
use iced::window::Direction;

use crate::app::App;
use crate::commands::AppCommand;
use crate::layout::screen_size_from_window;
use crate::services;
use crate::shell_integration::{
    integrated_shell_session, shell_integration_dir,
};
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::containers::event::ContainersEvent;
use crate::widgets::explorer::event::ExplorerEvent;
use crate::widgets::modes::event::ModesEvent;
use crate::widgets::notifications::event::NotificationsEvent;
use crate::widgets::palette::event::PaletteEvent;
use crate::widgets::quick_launch::event::QuickLaunchEvent;
use crate::widgets::settings::event::SettingsEvent;
use crate::widgets::sidebar::event::SidebarEvent;
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
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
    IcedReady,
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

/// Route an app event to the handler of the widget it belongs to.
pub(crate) fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => {
            let shell_path = app.widgets.settings.saved().terminal_shell();
            app.shell_session =
                integrated_shell_session(shell_path, &shell_integration_dir());
            app.host_name = services::read_host_name();

            Task::done(AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenTerminalTab,
            )))
        },
        AppEvent::Sidebar(event) => sidebar::route(app, event),
        AppEvent::Chrome(event) => chrome::route(app, event),
        AppEvent::Tabs(event) => tabs::route(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::route(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::route(app, event)
        },
        AppEvent::Explorer(event) => explorer::route(app, event),
        AppEvent::Settings(event) => settings::route(app, event),
        AppEvent::Palette(event) => palette::route(app, event),
        AppEvent::Modes(event) => modes::route(app, event),
        AppEvent::Containers(event) => containers::route(app, event),
        AppEvent::Notifications(event) => notifications::route(app, event),
        AppEvent::Command(command) => command::route(app, command),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state.set_screen_size(screen_size_from_window(size));

            terminal_workspace::route(
                app,
                TerminalWorkspaceEvent::Intent(
                    TerminalWorkspaceIntent::SyncPaneGridSize,
                ),
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

    use super::*;
    use crate::widgets::sidebar::event::SidebarIntent;

    #[test]
    fn host_name_is_read_when_the_window_is_ready_not_at_start() {
        let (mut app, _) = App::new();

        assert_eq!(app.host_name, None);

        let _first_tab = route(&mut app, AppEvent::IcedReady);

        assert_eq!(app.host_name, services::read_host_name());
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
        let _resized = route(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _open = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);

        let _enter = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        let _resized = route(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _leave = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);
    }
}
