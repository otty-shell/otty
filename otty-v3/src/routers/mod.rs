use iced::Task;

use crate::app::{App, AppEvent};

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
    use AppEvent::*;

    match event {
        IcedReady => flow::tabs::open_terminal_tab(app),
        // Sidebar widget
        SidebarUi(event) => sidebar::route_event(app, event),
        SidebarEffect(event) => sidebar::route_effect(event),
        SidebarCommand(command) => {
            sidebar::route_command(app, command)
        },
        // Chrome widget
        ChromeUi(event) => chrome::route_event(app, event),
        ChromeEffect(effect) => chrome::route_effect(effect),
        ChromeCommand(command) => {
            chrome::route_command(app, command)
        },
        // Tabs widget
        TabsUi(event) => tabs::route_event(app, event),
        TabsEffect(effect) => {
            tabs::route_effect(app, effect)
        },
        TabsCommand(command) => {
            tabs::route_command(app, command)
        },
        // Quick Launch widget
        QuickLaunch(event) => {
            quick_launch::route(app, event)
        },
        // Terminal Workspace widget
        TerminalWorkspaceUi(event) => {
            terminal_workspace::route_event(app, event)
        },
        TerminalWorkspaceEffect(effect) => {
            terminal_workspace::route_effect(app, effect)
        },
        TerminalWorkspaceCommand(command) => {
            terminal_workspace::route_command(app, command)
        },
        // Explorer widget
        ExplorerUi(event) => {
            explorer::route_event(app, event)
        },
        ExplorerEffect(effect) => {
            explorer::route_effect(effect)
        },
        ExplorerCommand(command) => {
            explorer::route_command(app, command)
        },
        // Settings widget
        SettingsUi(event) => {
            settings::route_event(app, event)
        },
        SettingsEffect(effect) => {
            settings::route_effect(app, effect)
        },
        SettingsCommand(command) => {
            settings::route_command(app, command)
        },
        // Cross-widget workflows
        OpenTerminalTab => {
            flow::tabs::open_terminal_tab(app)
        },
        OpenSettingsTab => {
            flow::tabs::open_settings_tab(app)
        },
        OpenQuickLaunchWizardCreateTab { parent_path } => {
            flow::quick_launch::open_wizard_create_tab(
                app,
                parent_path,
            )
        },
        OpenQuickLaunchWizardEditTab { path, command } => {
            flow::quick_launch::open_wizard_edit_tab(
                app, path, command,
            )
        },
        OpenQuickLaunchCommandTerminalTab {
            title,
            settings,
            command,
        } => flow::quick_launch::open_command_terminal_tab(
            app, title, settings, *command,
        ),
        OpenQuickLaunchErrorTab { title, message } => {
            flow::quick_launch::open_error_tab(app, title, message)
        },
        OpenFileTerminalTab { file_path } => {
            flow::tabs::open_file_terminal_tab(app, file_path)
        },
        CloseTab { tab_id } => {
            flow::quick_launch::close_tab(app, tab_id)
        },
        // Direct operations
        SyncTerminalGridSizes => {
            window::sync_terminal_grid_sizes(app);
            Task::none()
        },
        Keyboard(_event) => Task::none(),
        Window(iced::window::Event::Resized(size)) => {
            window::handle_resize(app, size)
        },
        Window(_) => Task::none(),
        ResizeWindow(dir) => window::handle_drag_resize(dir),
    }
}
