use iced::Task;

use super::event::{ChromeEffect, ChromeEvent, ChromeIntent};
use super::state::ChromeState;

/// Reduce a chrome intent event into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut ChromeState,
    event: ChromeIntent,
) -> Task<ChromeEvent> {
    match event {
        ChromeIntent::ToggleMaximize => {
            // A double click must not also start a drag, or the window
            // manager keeps the window where the drag holds it.
            state.release_title_bar();
            Task::done(ChromeEvent::Effect(ChromeEffect::ToggleMaximize))
        },
        ChromeIntent::MinimizeWindow => {
            Task::done(ChromeEvent::Effect(ChromeEffect::MinimizeWindow))
        },
        ChromeIntent::CloseWindow => {
            Task::done(ChromeEvent::Effect(ChromeEffect::CloseWindow))
        },
        ChromeIntent::ToggleLeftPanel => {
            Task::done(ChromeEvent::Effect(ChromeEffect::ToggleLeftPanel))
        },
        ChromeIntent::ToggleFilesPanel => {
            Task::done(ChromeEvent::Effect(ChromeEffect::ToggleFilesPanel))
        },
        ChromeIntent::ToggleFocusMode => {
            Task::done(ChromeEvent::Effect(ChromeEffect::ToggleFocusMode))
        },
        ChromeIntent::OpenSettings => {
            Task::done(ChromeEvent::Effect(ChromeEffect::OpenSettings))
        },
        ChromeIntent::OpenCommandPalette => {
            Task::done(ChromeEvent::Effect(ChromeEffect::OpenCommandPalette))
        },
        ChromeIntent::PressTitleBar => {
            state.press_title_bar();
            Task::none()
        },
        ChromeIntent::MoveOnTitleBar => {
            if !state.is_title_bar_pressed() {
                return Task::none();
            }

            state.release_title_bar();
            Task::done(ChromeEvent::Effect(ChromeEffect::StartWindowDrag))
        },
        ChromeIntent::ReleaseTitleBar => {
            state.release_title_bar();
            Task::none()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::task_outputs::collect_outputs;

    #[test]
    fn maximize_button_asks_to_toggle_maximize() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::ToggleMaximize,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleMaximize)]
        ));
    }

    #[test]
    fn given_pressed_title_bar_when_pointer_moves_then_window_drag_starts_once()
    {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);

        let first =
            collect_outputs(reduce(&mut state, ChromeIntent::MoveOnTitleBar));
        let second =
            collect_outputs(reduce(&mut state, ChromeIntent::MoveOnTitleBar));

        assert!(matches!(
            first.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::StartWindowDrag)]
        ));
        assert!(second.is_empty());
    }

    #[test]
    fn given_released_title_bar_when_pointer_moves_then_window_stays() {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);
        let _ = reduce(&mut state, ChromeIntent::ReleaseTitleBar);

        let events =
            collect_outputs(reduce(&mut state, ChromeIntent::MoveOnTitleBar));

        assert!(events.is_empty());
        assert!(!state.is_title_bar_pressed());
    }

    #[test]
    fn given_double_click_when_pointer_moves_then_window_only_maximizes() {
        let mut state = ChromeState::default();
        let _ = reduce(&mut state, ChromeIntent::PressTitleBar);

        let maximize =
            collect_outputs(reduce(&mut state, ChromeIntent::ToggleMaximize));
        let moved =
            collect_outputs(reduce(&mut state, ChromeIntent::MoveOnTitleBar));

        assert!(matches!(
            maximize.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleMaximize)]
        ));
        assert!(moved.is_empty());
    }

    #[test]
    fn files_panel_button_asks_to_toggle_the_files_panel() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::ToggleFilesPanel,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleFilesPanel)]
        ));
    }

    #[test]
    fn settings_button_asks_to_open_settings() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::OpenSettings,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::OpenSettings)]
        ));
    }

    #[test]
    fn left_panel_button_asks_to_toggle_the_left_panel() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::ToggleLeftPanel,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleLeftPanel)]
        ));
    }

    #[test]
    fn title_search_asks_to_open_the_command_palette() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::OpenCommandPalette,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::OpenCommandPalette)]
        ));
    }

    #[test]
    fn focus_button_asks_to_toggle_focus_mode() {
        let events = collect_outputs(reduce(
            &mut ChromeState::default(),
            ChromeIntent::ToggleFocusMode,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleFocusMode)]
        ));
    }
}
