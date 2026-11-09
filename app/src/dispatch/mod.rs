//! Delivery, part of the App layer: holds [`AppEvent`] and hands each event
//! to the one that handles it. `dispatch/x.rs` delivers the intents of
//! widget `x` to its reducer; the effects go to `effects/x.rs`.

use iced::Task;
#[cfg(not(target_os = "macos"))]
use iced::window;
use iced::window::Direction;

use crate::app::App;
use crate::commands::AppCommand;
use crate::effects;
use crate::layout::screen_size_from_window;
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
    IcedReady,
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
            effects::settings::run(app, effect)
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
        AppEvent::Command(command) => command::deliver(app, command),
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

    use super::*;
    use crate::services;
    use crate::widgets::sidebar::event::SidebarIntent;

    #[test]
    fn host_name_is_read_when_the_window_is_ready_not_at_start() {
        let (mut app, _) = App::new();

        assert_eq!(app.host_name, None);

        let _first_tab = dispatch(&mut app, AppEvent::IcedReady);

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
}
