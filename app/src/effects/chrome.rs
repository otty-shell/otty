use iced::{Task, window};

use crate::commands::AppCommand;
use crate::dispatch::AppEvent;
use crate::widgets::chrome::event::ChromeEffect;

/// Run a chrome effect: move the window, or turn a button into the app
/// command it stands for.
pub(crate) fn run(effect: ChromeEffect) -> Task<AppEvent> {
    use ChromeEffect::*;

    match effect {
        ToggleMaximize => window::latest().and_then(window::toggle_maximize),
        MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        CloseWindow => window::latest().and_then(window::close),
        StartWindowDrag => window::latest().and_then(window::drag),
        ToggleLeftPanel => command(AppCommand::ToggleLeftPanel),
        ToggleFocusMode => command(AppCommand::ToggleFocusMode),
        ToggleFilesPanel => command(AppCommand::ToggleFilesPanel),
        OpenSettings => command(AppCommand::OpenSettings),
        OpenNotifications => command(AppCommand::OpenNotifications),
        OpenCommandPalette => command(AppCommand::OpenPalette),
    }
}

fn command(command: AppCommand) -> Task<AppEvent> {
    Task::done(AppEvent::Command(command))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    fn command_of(effect: ChromeEffect) -> Option<AppCommand> {
        match collect_outputs(run(effect)).as_slice() {
            [AppEvent::Command(command)] => Some(*command),
            _ => None,
        }
    }

    #[test]
    fn left_panel_button_runs_the_left_panel_command() {
        assert_eq!(
            command_of(ChromeEffect::ToggleLeftPanel),
            Some(AppCommand::ToggleLeftPanel)
        );
    }

    #[test]
    fn files_panel_button_runs_the_files_panel_command() {
        assert_eq!(
            command_of(ChromeEffect::ToggleFilesPanel),
            Some(AppCommand::ToggleFilesPanel)
        );
    }

    #[test]
    fn focus_button_runs_the_focus_mode_command() {
        assert_eq!(
            command_of(ChromeEffect::ToggleFocusMode),
            Some(AppCommand::ToggleFocusMode)
        );
    }

    #[test]
    fn title_search_runs_the_open_palette_command() {
        assert_eq!(
            command_of(ChromeEffect::OpenCommandPalette),
            Some(AppCommand::OpenPalette)
        );
    }

    #[test]
    fn bell_runs_the_open_notifications_command() {
        assert_eq!(
            command_of(ChromeEffect::OpenNotifications),
            Some(AppCommand::OpenNotifications)
        );
    }

    #[test]
    fn settings_button_runs_the_open_settings_command() {
        assert_eq!(
            command_of(ChromeEffect::OpenSettings),
            Some(AppCommand::OpenSettings)
        );
    }
}
