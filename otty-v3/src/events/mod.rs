use iced::window::Direction;
use iced::{Task, window};

use crate::app::App;
use crate::layout::screen_size_from_window;
use crate::widgets::chrome::ChromeEvent;
use crate::widgets::explorer::ExplorerEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;
use crate::widgets::settings::SettingsEvent;
use crate::widgets::sidebar::SidebarEvent;
use crate::widgets::tabs::{TabsEvent, TabsUiEvent};
use crate::widgets::terminal_workspace::{
    TerminalWorkspaceEvent, TerminalWorkspaceUiEvent,
};

pub(crate) mod chrome;
pub(crate) mod explorer;
pub(crate) mod flow;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

/// App-wide events that drive the root update loop.
#[derive(Clone)]
pub(crate) enum AppEvent {
    IcedReady,
    // Sidebar widget
    Sidebar(SidebarEvent),
    // Chrome widget
    Chrome(ChromeEvent),
    // Tabs widget
    Tabs(TabsEvent),
    // Quick Launch widget
    QuickLaunch(QuickLaunchEvent),
    // Terminal Workspace widget
    TerminalWorkspace(TerminalWorkspaceEvent),
    // Explorer widget
    Explorer(ExplorerEvent),
    // Settings widget
    Settings(SettingsEvent),
    // Cross-widget workflows
    OpenTerminalTab,
    OpenFileTerminalTab { file_path: std::path::PathBuf },
    CloseTab { tab_id: u64 },
    // Direct operations
    SyncTerminalGridSizes,
    Keyboard(iced::keyboard::Event),
    Window(iced::window::Event),
    ResizeWindow(Direction),
}

pub(crate) fn handle(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => flow::tabs::open_terminal_tab(app),
        AppEvent::Sidebar(event) => sidebar::handle(app, event),
        AppEvent::Chrome(event) => chrome::handle(app, event),
        AppEvent::Tabs(event) => tabs::handle(app, event),
        AppEvent::QuickLaunch(event) => quick_launch::handle(app, event),
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::handle(app, event)
        },
        AppEvent::Explorer(event) => explorer::handle(app, event),
        AppEvent::Settings(event) => settings::handle(app, event),
        AppEvent::OpenTerminalTab => flow::tabs::open_terminal_tab(app),
        AppEvent::OpenFileTerminalTab { file_path } => {
            flow::tabs::open_file_terminal_tab(app, file_path)
        },
        AppEvent::CloseTab { tab_id } => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::CloseTab {
                tab_id,
            })))
        },
        AppEvent::SyncTerminalGridSizes => sync_terminal_grid_sizes(),
        AppEvent::Keyboard(_event) => Task::none(),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            handle_resize(app, size)
        },
        AppEvent::ResizeWindow(dir) => {
            window::latest().and_then(move |id| window::drag_resize(id, dir))
        },
        AppEvent::Window(_) => Task::none(),
    }
}

fn handle_resize(app: &mut App, size: iced::Size) -> Task<AppEvent> {
    app.window_size = size;
    app.state.window_size = size;
    app.state.set_screen_size(screen_size_from_window(size));

    sync_terminal_grid_sizes()
}

fn sync_terminal_grid_sizes() -> Task<AppEvent> {
    Task::done(AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Ui(
        TerminalWorkspaceUiEvent::SyncPaneGridSize,
    )))
}
