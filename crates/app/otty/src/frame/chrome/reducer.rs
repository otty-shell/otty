#![forbid(clippy::disallowed_methods)]

use otty_app_domain::commands::AppCommand;

use super::event::{ChromeIntent, ChromeOutcome};
use super::state::ChromeState;

/// Reduce a chrome intent into state changes and what the frame does next:
/// a header button reports the app command it stands for, a window button
/// the window operation.
pub(super) fn reduce(
    state: &mut ChromeState,
    intent: ChromeIntent,
) -> Vec<ChromeOutcome> {
    match intent {
        ChromeIntent::ToggleMaximize => {
            // A double click must not also start a drag, or the window
            // manager keeps the window where the drag holds it.
            state.release_title_bar();
            vec![ChromeOutcome::ToggleMaximize]
        },
        ChromeIntent::MinimizeWindow => vec![ChromeOutcome::MinimizeWindow],
        ChromeIntent::CloseWindow => vec![ChromeOutcome::CloseWindow],
        ChromeIntent::ToggleLeftPanel => command(AppCommand::ToggleLeftPanel),
        ChromeIntent::ToggleFilesPanel => command(AppCommand::ToggleFilesPanel),
        ChromeIntent::ToggleFocusMode => command(AppCommand::ToggleFocusMode),
        ChromeIntent::OpenSettings => command(AppCommand::OpenSettings),
        ChromeIntent::OpenNotifications => {
            command(AppCommand::OpenNotifications)
        },
        ChromeIntent::OpenCommandPalette => command(AppCommand::OpenPalette),
        ChromeIntent::PressTitleBar => {
            state.press_title_bar();
            Vec::new()
        },
        ChromeIntent::MoveOnTitleBar => {
            if !state.is_title_bar_pressed() {
                return Vec::new();
            }

            state.release_title_bar();
            vec![ChromeOutcome::StartWindowDrag]
        },
        ChromeIntent::ReleaseTitleBar => {
            state.release_title_bar();
            Vec::new()
        },
    }
}

/// A header button that does what `command` does.
fn command(command: AppCommand) -> Vec<ChromeOutcome> {
    vec![ChromeOutcome::Command(command)]
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::AppCommand;

    use super::*;

    #[test]
    fn maximize_button_asks_to_toggle_maximize() {
        let outcomes =
            reduce(&mut ChromeState::default(), ChromeIntent::ToggleMaximize);

        assert_eq!(outcomes, vec![ChromeOutcome::ToggleMaximize]);
    }

    #[test]
    fn minimize_and_close_buttons_ask_for_their_window_operation() {
        let minimize =
            reduce(&mut ChromeState::default(), ChromeIntent::MinimizeWindow);
        let close =
            reduce(&mut ChromeState::default(), ChromeIntent::CloseWindow);

        assert_eq!(minimize, vec![ChromeOutcome::MinimizeWindow]);
        assert_eq!(close, vec![ChromeOutcome::CloseWindow]);
    }

    #[test]
    fn given_pressed_title_bar_when_pointer_moves_then_window_drag_starts_once()
    {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);

        let first = reduce(&mut state, ChromeIntent::MoveOnTitleBar);
        let second = reduce(&mut state, ChromeIntent::MoveOnTitleBar);

        assert_eq!(first, vec![ChromeOutcome::StartWindowDrag]);
        assert!(second.is_empty());
    }

    #[test]
    fn given_released_title_bar_when_pointer_moves_then_window_stays() {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);
        let _ = reduce(&mut state, ChromeIntent::ReleaseTitleBar);

        let outcomes = reduce(&mut state, ChromeIntent::MoveOnTitleBar);

        assert!(outcomes.is_empty());
        assert!(!state.is_title_bar_pressed());
    }

    #[test]
    fn given_double_click_when_pointer_moves_then_window_only_maximizes() {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);

        let maximize = reduce(&mut state, ChromeIntent::ToggleMaximize);
        let moved = reduce(&mut state, ChromeIntent::MoveOnTitleBar);

        assert_eq!(maximize, vec![ChromeOutcome::ToggleMaximize]);
        assert!(moved.is_empty());
    }

    #[test]
    fn left_panel_button_runs_the_left_panel_command() {
        let outcomes =
            reduce(&mut ChromeState::default(), ChromeIntent::ToggleLeftPanel);

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::ToggleLeftPanel)]
        );
    }

    #[test]
    fn files_panel_button_runs_the_files_panel_command() {
        let outcomes =
            reduce(&mut ChromeState::default(), ChromeIntent::ToggleFilesPanel);

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::ToggleFilesPanel)]
        );
    }

    #[test]
    fn focus_button_runs_the_focus_mode_command() {
        let outcomes =
            reduce(&mut ChromeState::default(), ChromeIntent::ToggleFocusMode);

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::ToggleFocusMode)]
        );
    }

    #[test]
    fn settings_button_runs_the_open_settings_command() {
        let outcomes =
            reduce(&mut ChromeState::default(), ChromeIntent::OpenSettings);

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::OpenSettings)]
        );
    }

    #[test]
    fn bell_runs_the_open_notifications_command() {
        let outcomes = reduce(
            &mut ChromeState::default(),
            ChromeIntent::OpenNotifications,
        );

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::OpenNotifications)]
        );
    }

    #[test]
    fn title_search_runs_the_open_palette_command() {
        let outcomes = reduce(
            &mut ChromeState::default(),
            ChromeIntent::OpenCommandPalette,
        );

        assert_eq!(
            outcomes,
            vec![ChromeOutcome::Command(AppCommand::OpenPalette)]
        );
    }
}
