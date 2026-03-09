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
        ChromeIntent::ToggleSidebarVisibility => Task::done(
            ChromeEvent::Effect(ChromeEffect::ToggleSidebarVisibility),
        ),
        ChromeIntent::StartWindowDrag => {
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
}
