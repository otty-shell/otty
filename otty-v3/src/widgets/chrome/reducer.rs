use iced::{Task, window};

use super::event::{ChromeEffect, ChromeEvent, ChromeUiEvent};
use super::state::ChromeState;

/// Reduce a chrome UI event into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut ChromeState,
    event: ChromeUiEvent,
) -> Task<ChromeEvent> {
    match event {
        ChromeUiEvent::ToggleFullScreen => {
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
        ChromeUiEvent::MinimizeWindow => {
            Task::done(ChromeEvent::Effect(ChromeEffect::MinimizeWindow))
        },
        ChromeUiEvent::CloseWindow => {
            Task::done(ChromeEvent::Effect(ChromeEffect::CloseWindow))
        },
        ChromeUiEvent::ToggleSidebarVisibility => Task::done(
            ChromeEvent::Effect(ChromeEffect::ToggleSidebarVisibility),
        ),
        ChromeUiEvent::StartWindowDrag => {
            Task::done(ChromeEvent::Effect(ChromeEffect::StartWindowDrag))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_fullscreen_flips_state() {
        let mut state = ChromeState::default();
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeUiEvent::ToggleFullScreen);
        assert!(state.is_fullscreen());

        let _ = reduce(&mut state, ChromeUiEvent::ToggleFullScreen);
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn non_fullscreen_commands_preserve_state() {
        let mut state = ChromeState::default();

        let _ = reduce(&mut state, ChromeUiEvent::MinimizeWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeUiEvent::CloseWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeUiEvent::StartWindowDrag);
        assert!(!state.is_fullscreen());
    }
}
