use iced::{Task, window};

use super::AppEvent;
use crate::app::App;
use crate::commands::AppCommand;
use crate::widgets::chrome::{ChromeEffect, ChromeEvent};

/// Route a chrome event: reduce its intents, run the window effects and
/// turn the button effects into the app commands they stand for.
pub(crate) fn route(app: &mut App, event: ChromeEvent) -> Task<AppEvent> {
    match event {
        ChromeEvent::Intent(event) => {
            app.widgets.chrome.reduce(event).map(AppEvent::Chrome)
        },
        ChromeEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: ChromeEffect) -> Task<AppEvent> {
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
        match collect_outputs(handle_effect(effect)).as_slice() {
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
    fn settings_button_runs_the_open_settings_command() {
        assert_eq!(
            command_of(ChromeEffect::OpenSettings),
            Some(AppCommand::OpenSettings)
        );
    }
}
