use iced::Task;

use crate::app::{App, AppEvent};
use crate::widgets::tabs::TabsCommand;

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
        AppEvent::SidebarUi(event) => sidebar::route_event(app, event),
        AppEvent::SidebarEffect(event) => sidebar::route_effect(event),
        AppEvent::SidebarCommand(command) => {
            sidebar::route_command(app, command)
        },
        // Chrome widget
        AppEvent::ChromeUi(event) => chrome::route_event(app, event),
        AppEvent::ChromeEffect(effect) => chrome::route_effect(effect),
        AppEvent::ChromeCommand(command) => {
            chrome::route_command(app, command)
        },
        // Tabs widget
        AppEvent::TabsUi(event) => tabs::route_event(app, event),
        AppEvent::TabsEffect(effect) => {
            tabs::route_effect(app, effect)
        },
        AppEvent::TabsCommand(command) => {
            tabs::route_command(app, command)
        },
        // Quick Launch widget
        AppEvent::QuickLaunch(event) => {
            quick_launch::route(app, event)
        },
        // Terminal Workspace widget
        AppEvent::TerminalWorkspaceUi(event) => {
            terminal_workspace::route_event(app, event)
        },
        AppEvent::TerminalWorkspaceEffect(effect) => {
            terminal_workspace::route_effect(app, effect)
        },
        AppEvent::TerminalWorkspaceCommand(command) => {
            terminal_workspace::route_command(app, command)
        },
        // Explorer widget
        AppEvent::ExplorerUi(event) => {
            explorer::route_event(app, event)
        },
        AppEvent::ExplorerEffect(effect) => {
            explorer::route_effect(effect)
        },
        AppEvent::ExplorerCommand(command) => {
            explorer::route_command(app, command)
        },
        // Settings widget
        AppEvent::SettingsUi(event) => {
            settings::route_event(app, event)
        },
        AppEvent::SettingsEffect(effect) => {
            settings::route_effect(app, effect)
        },
        AppEvent::SettingsCommand(command) => {
            settings::route_command(app, command)
        },
        // Cross-widget workflows
        AppEvent::OpenTerminalTab => {
            flow::tabs::open_terminal_tab(app)
        },
        AppEvent::OpenSettingsTab => {
            flow::tabs::open_settings_tab(app)
        },
        AppEvent::OpenFileTerminalTab { file_path } => {
            flow::tabs::open_file_terminal_tab(app, file_path)
        },
        AppEvent::CloseTab { tab_id } => {
            Task::done(AppEvent::TabsCommand(TabsCommand::Close { tab_id }))
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
