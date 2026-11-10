use iced::{Task, window};
use otty_app_slots::Component;

use super::command;
use crate::app::App;
use crate::chrome::{ChromeIntent, ChromeOutcome};
use crate::message::AppEvent;

/// Give a chrome intent to the widget and read what it reports: a header
/// button runs its command now, a window button becomes a window operation.
pub(crate) fn deliver(app: &mut App, intent: ChromeIntent) -> Task<AppEvent> {
    let outcomes = app.chrome.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: ChromeOutcome) -> Task<AppEvent> {
    match outcome {
        ChromeOutcome::Command(asked) => command::deliver(app, asked),
        ChromeOutcome::ToggleMaximize => {
            window::latest().and_then(window::toggle_maximize)
        },
        ChromeOutcome::MinimizeWindow => {
            window::latest().and_then(|id| window::minimize(id, true))
        },
        ChromeOutcome::CloseWindow => window::latest().and_then(window::close),
        ChromeOutcome::StartWindowDrag => {
            window::latest().and_then(window::drag)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::window::app;

    /// A header button runs its command in the same update: no message goes
    /// through iced in between.
    #[test]
    fn a_panel_button_toggles_its_panel_in_the_same_update() {
        let mut app = app();
        let before = app.sidebar.vm();

        let _left = deliver(&mut app, ChromeIntent::ToggleLeftPanel);
        let _files = deliver(&mut app, ChromeIntent::ToggleFilesPanel);
        let _focus = deliver(&mut app, ChromeIntent::ToggleFocusMode);

        let after = app.sidebar.vm();
        assert_ne!(after.is_left_open, before.is_left_open);
        assert_ne!(after.is_right_open, before.is_right_open);
        assert!(after.is_focus_mode);
    }
}
