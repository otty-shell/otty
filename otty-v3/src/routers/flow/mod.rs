pub(crate) mod quick_launch;
pub(crate) mod tabs;

use iced::Task;

use crate::app::{App, AppEvent, AppFlowEvent};

/// Route a flow event to the appropriate cross-widget orchestrator.
pub(crate) fn route(app: &mut App, event: AppFlowEvent) -> Task<AppEvent> {
    match event {
        AppFlowEvent::OpenTerminalTab => tabs::open_terminal_tab(app),
        AppFlowEvent::OpenSettingsTab => tabs::open_settings_tab(app),
        AppFlowEvent::OpenQuickLaunchWizardCreateTab { parent_path } => {
            quick_launch::open_wizard_create_tab(app, parent_path)
        },
        AppFlowEvent::OpenQuickLaunchWizardEditTab { path, command } => {
            quick_launch::open_wizard_edit_tab(app, path, command)
        },
        AppFlowEvent::OpenQuickLaunchCommandTerminalTab {
            title,
            settings,
            command,
        } => quick_launch::open_command_terminal_tab(
            app, title, settings, *command,
        ),
        AppFlowEvent::OpenQuickLaunchErrorTab { title, message } => {
            quick_launch::open_error_tab(app, title, message)
        },
        AppFlowEvent::OpenFileTerminalTab { file_path } => {
            tabs::open_file_terminal_tab(app, file_path)
        },
        AppFlowEvent::CloseTab { tab_id } => {
            quick_launch::close_tab(app, tab_id)
        },
    }
}
