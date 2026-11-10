#![forbid(clippy::disallowed_methods)]

use otty_app_domain::commands::AppCommand;

use super::event::{TitleBarIntent, TitleBarOutcome};
use super::state::TitleBarState;

/// Reduce a title bar intent into state changes and what the frame does next:
/// a header button reports the app command it stands for, a window button
/// the window operation.
pub(super) fn reduce(
    state: &mut TitleBarState,
    intent: TitleBarIntent,
) -> Vec<TitleBarOutcome> {
    match intent {
        TitleBarIntent::ToggleMaximize => {
            // A double click must not also start a drag, or the window
            // manager keeps the window where the drag holds it.
            state.release_title_bar();
            vec![TitleBarOutcome::ToggleMaximize]
        },
        TitleBarIntent::MinimizeWindow => vec![TitleBarOutcome::MinimizeWindow],
        TitleBarIntent::CloseWindow => vec![TitleBarOutcome::CloseWindow],
        TitleBarIntent::ToggleLeftPanel => command(AppCommand::ToggleLeftPanel),
        TitleBarIntent::ToggleFilesPanel => {
            command(AppCommand::ToggleFilesPanel)
        },
        TitleBarIntent::ToggleFocusMode => command(AppCommand::ToggleFocusMode),
        TitleBarIntent::OpenSettings => command(AppCommand::OpenSettings),
        TitleBarIntent::OpenNotifications => {
            command(AppCommand::OpenNotifications)
        },
        TitleBarIntent::OpenCommandPalette => command(AppCommand::OpenPalette),
        TitleBarIntent::PressTitleBar => {
            state.press_title_bar();
            Vec::new()
        },
        TitleBarIntent::MoveOnTitleBar => {
            if !state.is_title_bar_pressed() {
                return Vec::new();
            }

            state.release_title_bar();
            vec![TitleBarOutcome::StartWindowDrag]
        },
        TitleBarIntent::ReleaseTitleBar => {
            state.release_title_bar();
            Vec::new()
        },
    }
}

/// A header button that does what `command` does.
fn command(command: AppCommand) -> Vec<TitleBarOutcome> {
    vec![TitleBarOutcome::Command(command)]
}

#[cfg(test)]
mod tests {
    use otty_app_domain::commands::AppCommand;

    use super::*;

    #[test]
    fn maximize_button_asks_to_toggle_maximize() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::ToggleMaximize,
        );

        assert_eq!(outcomes, vec![TitleBarOutcome::ToggleMaximize]);
    }

    #[test]
    fn minimize_and_close_buttons_ask_for_their_window_operation() {
        let minimize = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::MinimizeWindow,
        );
        let close =
            reduce(&mut TitleBarState::default(), TitleBarIntent::CloseWindow);

        assert_eq!(minimize, vec![TitleBarOutcome::MinimizeWindow]);
        assert_eq!(close, vec![TitleBarOutcome::CloseWindow]);
    }

    #[test]
    fn given_pressed_title_bar_when_pointer_moves_then_window_drag_starts_once()
    {
        let mut state = TitleBarState::default();
        let _ = reduce(&mut state, TitleBarIntent::PressTitleBar);

        let first = reduce(&mut state, TitleBarIntent::MoveOnTitleBar);
        let second = reduce(&mut state, TitleBarIntent::MoveOnTitleBar);

        assert_eq!(first, vec![TitleBarOutcome::StartWindowDrag]);
        assert!(second.is_empty());
    }

    #[test]
    fn given_released_title_bar_when_pointer_moves_then_window_stays() {
        let mut state = TitleBarState::default();
        let _ = reduce(&mut state, TitleBarIntent::PressTitleBar);
        let _ = reduce(&mut state, TitleBarIntent::ReleaseTitleBar);

        let outcomes = reduce(&mut state, TitleBarIntent::MoveOnTitleBar);

        assert!(outcomes.is_empty());
        assert!(!state.is_title_bar_pressed());
    }

    #[test]
    fn given_double_click_when_pointer_moves_then_window_only_maximizes() {
        let mut state = TitleBarState::default();
        let _ = reduce(&mut state, TitleBarIntent::PressTitleBar);

        let maximize = reduce(&mut state, TitleBarIntent::ToggleMaximize);
        let moved = reduce(&mut state, TitleBarIntent::MoveOnTitleBar);

        assert_eq!(maximize, vec![TitleBarOutcome::ToggleMaximize]);
        assert!(moved.is_empty());
    }

    #[test]
    fn left_panel_button_runs_the_left_panel_command() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::ToggleLeftPanel,
        );

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::ToggleLeftPanel)]
        );
    }

    #[test]
    fn files_panel_button_runs_the_files_panel_command() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::ToggleFilesPanel,
        );

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::ToggleFilesPanel)]
        );
    }

    #[test]
    fn focus_button_runs_the_focus_mode_command() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::ToggleFocusMode,
        );

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::ToggleFocusMode)]
        );
    }

    #[test]
    fn settings_button_runs_the_open_settings_command() {
        let outcomes =
            reduce(&mut TitleBarState::default(), TitleBarIntent::OpenSettings);

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::OpenSettings)]
        );
    }

    #[test]
    fn bell_runs_the_open_notifications_command() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::OpenNotifications,
        );

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::OpenNotifications)]
        );
    }

    #[test]
    fn title_search_runs_the_open_palette_command() {
        let outcomes = reduce(
            &mut TitleBarState::default(),
            TitleBarIntent::OpenCommandPalette,
        );

        assert_eq!(
            outcomes,
            vec![TitleBarOutcome::Command(AppCommand::OpenPalette)]
        );
    }
}
