use iced::Task;
#[cfg(not(target_os = "macos"))]
use iced::window;
use iced::window::Direction;

use crate::app::App;
use crate::layout::screen_size_from_window;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::docker::DockerEvent;
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::modes::ModesEvent;
use crate::widgets::palette::PaletteEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::settings::SettingsEvent;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) mod chrome;
pub(crate) mod docker;
pub(crate) mod explorer;
pub(crate) mod modes;
pub(crate) mod palette;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod shortcuts;
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
    Docker(DockerEvent),
    IcedReady,
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

/// Route an app event to the handler of the widget it belongs to.
pub(crate) fn handle(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => Task::done(AppEvent::Tabs(TabsEvent::Intent(
            TabsIntent::OpenTerminalTab,
        ))),
        AppEvent::Sidebar(event) => sidebar::handle(app, event),
        AppEvent::Chrome(event) => chrome::handle(app, event),
        AppEvent::Tabs(event) => tabs::handle(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::handle(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::handle(app, event)
        },
        AppEvent::Explorer(event) => explorer::handle(app, event),
        AppEvent::Settings(event) => settings::handle(app, event),
        AppEvent::Palette(event) => palette::handle(app, event),
        AppEvent::Modes(event) => modes::handle(app, event),
        AppEvent::Docker(event) => docker::handle(app, event),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        // Only keys no widget captured arrive here, so a shortcut a focused
        // terminal already reported does not run twice.
        AppEvent::Keyboard(event) => shortcuts::handle_key_event(app, &event),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state.set_screen_size(screen_size_from_window(size));

            terminal_workspace::handle(
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
        AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{Event, Key, Location};
    use iced::widget::pane_grid;

    use super::*;
    use crate::shortcuts::{AppShortcut, Platform};
    use crate::testing::task_outputs::collect_outputs;
    use crate::widgets::sidebar::SidebarIntent;

    #[test]
    fn keyboard_event_runs_the_app_shortcut_it_stands_for() {
        let (mut app, _) = App::new();
        let keys = AppShortcut::Settings.keys(Platform::current());
        let event = Event::KeyPressed {
            key: Key::Character(keys.key().into()),
            modified_key: Key::Character(keys.key().into()),
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: keys.modifiers(),
            text: None,
            repeat: false,
        };

        let events =
            collect_outputs(handle(&mut app, AppEvent::Keyboard(event)));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Tabs(TabsEvent::Intent(
                TabsIntent::OpenSettingsTab
            ))]
        ));
    }

    /// Panel widths are shares of the window, so a smaller window keeps them,
    /// for a panel that is closed as for one hidden by focus mode.
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
        let _resized = handle(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _open = app.widgets.sidebar.reduce(SidebarIntent::ToggleLeft);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);

        let _enter = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);
        let _resized = handle(
            &mut app,
            AppEvent::Window(iced::window::Event::Resized(smaller)),
        );
        let _leave = app.widgets.sidebar.reduce(SidebarIntent::ToggleFocusMode);

        assert_eq!(app.widgets.sidebar.left_ratio(), 0.3);
    }
}
