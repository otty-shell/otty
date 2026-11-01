use iced::{Task, window};

use super::event::{ChromeEffect, ChromeEvent, ChromeIntent};
use super::state::ChromeState;

/// Reduce a chrome intent event into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut ChromeState,
    event: ChromeIntent,
) -> Task<ChromeEvent> {
    match event {
        ChromeIntent::ToggleFullScreen => {
            let is_fullscreen = state.toggle_fullscreen();
            let mode = if is_fullscreen {
                window::Mode::Fullscreen
            } else {
                window::Mode::Windowed
            };
            Task::done(ChromeEvent::Effect(ChromeEffect::FullScreenToggled {
                mode,
            }))
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
        ChromeIntent::StartWindowDrag => {
            Task::done(ChromeEvent::Effect(ChromeEffect::StartWindowDrag))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_outputs::collect_outputs;

    #[test]
    fn toggle_fullscreen_flips_state() {
        let mut state = ChromeState::default();
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeIntent::ToggleFullScreen);
        assert!(state.is_fullscreen());

        let _ = reduce(&mut state, ChromeIntent::ToggleFullScreen);
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn non_fullscreen_commands_preserve_state() {
        let mut state = ChromeState::default();

        let _ = reduce(&mut state, ChromeIntent::MinimizeWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeIntent::CloseWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeIntent::StartWindowDrag);
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn files_panel_button_asks_to_toggle_the_files_panel() {
        let mut state = ChromeState::default();

        let events =
            collect_outputs(reduce(&mut state, ChromeIntent::ToggleFilesPanel));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleFilesPanel)]
        ));
    }

    #[test]
    fn settings_button_asks_to_open_settings() {
        let mut state = ChromeState::default();

        let events =
            collect_outputs(reduce(&mut state, ChromeIntent::OpenSettings));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::OpenSettings)]
        ));
    }

    #[test]
    fn left_panel_button_asks_to_toggle_the_left_panel() {
        let mut state = ChromeState::default();

        let events =
            collect_outputs(reduce(&mut state, ChromeIntent::ToggleLeftPanel));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleLeftPanel)]
        ));
    }

    #[test]
    fn title_search_asks_to_open_the_command_palette() {
        let mut state = ChromeState::default();

        let events = collect_outputs(reduce(
            &mut state,
            ChromeIntent::OpenCommandPalette,
        ));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::OpenCommandPalette)]
        ));
    }

    #[test]
    fn focus_button_asks_to_toggle_focus_mode() {
        let mut state = ChromeState::default();

        let events =
            collect_outputs(reduce(&mut state, ChromeIntent::ToggleFocusMode));

        assert!(matches!(
            events.as_slice(),
            [ChromeEvent::Effect(ChromeEffect::ToggleFocusMode)]
        ));
    }
}
