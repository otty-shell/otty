use iced::{Task, window};

use super::command::ChromeCommand;
use super::event::ChromeEffect;
use super::state::ChromeState;

/// Reduce a chrome command into state mutation and effect tasks.
pub(crate) fn reduce(
    state: &mut ChromeState,
    command: ChromeCommand,
) -> Task<ChromeEffect> {
    match command {
        ChromeCommand::ToggleFullScreen => {
            let is_fullscreen = state.toggle_fullscreen();
            let mode = if is_fullscreen {
                window::Mode::Fullscreen
            } else {
                window::Mode::Windowed
            };
            Task::done(ChromeEffect::FullScreenToggled { mode })
        },
        ChromeCommand::MinimizeWindow => {
            Task::done(ChromeEffect::MinimizeWindow)
        },
        ChromeCommand::CloseWindow => Task::done(ChromeEffect::CloseWindow),
        ChromeCommand::ToggleSidebarVisibility => {
            Task::done(ChromeEffect::ToggleSidebarVisibility)
        },
        ChromeCommand::StartWindowDrag => {
            Task::done(ChromeEffect::StartWindowDrag)
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

        let _ = reduce(&mut state, ChromeCommand::ToggleFullScreen);
        assert!(state.is_fullscreen());

        let _ = reduce(&mut state, ChromeCommand::ToggleFullScreen);
        assert!(!state.is_fullscreen());
    }

    #[test]
    fn non_fullscreen_commands_preserve_state() {
        let mut state = ChromeState::default();

        let _ = reduce(&mut state, ChromeCommand::MinimizeWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeCommand::CloseWindow);
        assert!(!state.is_fullscreen());

        let _ = reduce(&mut state, ChromeCommand::StartWindowDrag);
        assert!(!state.is_fullscreen());
    }
}
