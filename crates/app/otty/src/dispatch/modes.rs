use iced::Task;
use otty_app_slots::Feature;

use crate::app::App;
use crate::effects;
use crate::message::AppEvent;
use crate::modes::{ModesIntent, ModesOutcome};

/// Give a modes intent to the widget and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: ModesIntent) -> Task<AppEvent> {
    let outcomes = app.modes.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes.into_iter().map(read).collect();

    Task::batch(tasks)
}

fn read(outcome: ModesOutcome) -> Task<AppEvent> {
    match outcome {
        // The frame reads the active mode when it draws. Back in Terminal
        // mode the terminal feature (issue 11) syncs its grid size and
        // focuses the active pane; the containers feature (issue 13) starts
        // talking to the engines in Containers mode and stops when it is
        // left.
        ModesOutcome::ModeChanged(_mode) => Task::none(),
        ModesOutcome::SavePinned(pinned) => effects::modes::save_pinned(pinned),
        ModesOutcome::MenuOpened => super::take_keyboard(),
        // Once the menu closes in Terminal mode, the terminal feature (issue
        // 11) gives the active pane its keys back. In another mode nothing
        // takes the keyboard.
        ModesOutcome::MenuClosed(_mode) => Task::none(),
        // The notifications feature (issue 06) shows the text as a toast.
        ModesOutcome::PinnedSaveFailed(_text) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;

    use super::*;
    use crate::testing::task_outputs::focuses_nothing;
    use crate::testing::window::app;

    #[test]
    fn an_intent_is_reduced_by_the_widget() {
        let mut app = app();

        let _task = deliver(&mut app, ModesIntent::Select(Mode::Containers));

        assert_eq!(app.modes.active(), Mode::Containers);
    }

    #[test]
    fn an_opened_menu_takes_the_keyboard_focus() {
        let mut app = app();

        let task = deliver(&mut app, ModesIntent::ToggleMenu);

        assert!(app.modes.is_menu_open());
        assert!(focuses_nothing(task));
    }

    /// After an overlay closes in Containers mode, nothing has the keyboard.
    #[test]
    fn a_menu_closed_in_containers_mode_focuses_nothing() {
        let mut app = app();
        let _select = deliver(&mut app, ModesIntent::Select(Mode::Containers));
        let _open = deliver(&mut app, ModesIntent::ToggleMenu);

        let task = deliver(&mut app, ModesIntent::CloseMenu);

        assert!(!app.modes.is_menu_open());
        assert_eq!(task.units(), 0);
    }
}
