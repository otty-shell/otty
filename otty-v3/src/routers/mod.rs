use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::tabs::{TabsEvent, TabsUiEvent};

pub(crate) mod chrome;
pub(crate) mod explorer;
pub(crate) mod flow;
pub(crate) mod quick_launch;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;
pub(crate) mod window;

pub(crate) fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => flow::tabs::open_terminal_tab(app),
        // Sidebar widget
        AppEvent::Sidebar(event) => sidebar::route(app, event),
        // Chrome widget
        AppEvent::Chrome(event) => chrome::route(app, event),
        // Tabs widget
        AppEvent::Tabs(event) => tabs::route(app, event),
        // Quick Launch widget
        AppEvent::QuickLaunch(event) => quick_launch::route(app, event),
        // Terminal Workspace widget
        AppEvent::TerminalWorkspace(event) => {
            terminal_workspace::route(app, event)
        },
        // Explorer widget
        AppEvent::Explorer(event) => explorer::route(app, event),
        // Settings widget
        AppEvent::Settings(event) => settings::route(app, event),
        // Cross-widget workflows
        AppEvent::OpenTerminalTab => flow::tabs::open_terminal_tab(app),
        AppEvent::OpenSettingsTab => flow::tabs::open_settings_tab(app),
        AppEvent::OpenFileTerminalTab { file_path } => {
            flow::tabs::open_file_terminal_tab(app, file_path)
        },
        AppEvent::CloseTab { tab_id } => {
            Task::done(AppEvent::Tabs(TabsEvent::Ui(TabsUiEvent::CloseTab {
                tab_id,
            })))
        },
        // Direct operations
        AppEvent::SyncTerminalGridSizes => {
            window::sync_terminal_grid_sizes(app);
            Task::none()
        },
        AppEvent::Keyboard(_event) => Task::none(),
        AppEvent::Window(iced::window::Event::Resized(size)) => {
            window::handle_resize(app, size)
        },
        AppEvent::Window(_) => Task::none(),
        AppEvent::ResizeWindow(dir) => window::handle_drag_resize(dir),
    }
}
