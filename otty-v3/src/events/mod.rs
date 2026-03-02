use iced::window::Direction;
use iced::{Task, window};

use crate::app::App;
use crate::layout::screen_size_from_window;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::settings::SettingsEvent;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::tabs::{TabsEvent, TabsIntent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) mod chrome;
pub(crate) mod explorer;
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
    OpenTerminalTab,
    IcedReady,
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

pub(crate) fn handle(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => open_terminal_tab(app),
        AppEvent::Sidebar(event) => sidebar::handle(app, event),
        AppEvent::Chrome(event) => chrome::handle(app, event),
        AppEvent::Tabs(event) => tabs::handle(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::handle(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::handle(app, event)
        },
        AppEvent::Explorer(event) => explorer::handle(app, event),
        AppEvent::Settings(event) => settings::handle(app, event),
        AppEvent::OpenTerminalTab => open_terminal_tab(app),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Keyboard(_event) => Task::none(),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            app.window_size = size;
            app.state.window_size = size;
            app.state.set_screen_size(screen_size_from_window(size));

            terminal_workspace::handle(app, TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            ))
        },
        AppEvent::ResizeWindow(dir) => {
            window::latest().and_then(move |id| window::drag_resize(id, dir))
        },
        AppEvent::Window(_) => Task::none(),
    }
}

fn open_terminal_tab(app: &mut App) -> Task<AppEvent> {
    let terminal_id = app.widgets.terminal_workspace.allocate_terminal_id();
    let title = app.shell_session.name().to_string();

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenTerminalTab { terminal_id, title },
    )))
}
