use iced::Task;
use otty_app_containers::ContainersIntent;
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Feature;
use otty_app_terminal::TerminalIntent;

use super::{containers, notifications, terminal};
use crate::app::App;
use crate::effects;
use crate::message::AppEvent;
use crate::modes::{ModesIntent, ModesOutcome};

/// Give a modes intent to the widget and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: ModesIntent) -> Task<AppEvent> {
    let outcomes = app.modes.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: ModesOutcome) -> Task<AppEvent> {
    match outcome {
        // The frame reads the active mode when it draws. Both features
        // decide: back in Terminal mode the terminal panes take the grid
        // size the window has now and the active pane its keys; the
        // containers feature talks to the engines in Containers mode only.
        ModesOutcome::ModeChanged(mode) => Task::batch([
            terminal::deliver(app, TerminalIntent::ModeChanged),
            containers::deliver(app, ContainersIntent::ModeChanged(mode)),
        ]),
        ModesOutcome::SavePinned(pinned) => effects::modes::save_pinned(pinned),
        ModesOutcome::MenuOpened => super::take_keyboard(),
        // The terminal decides: once the menu closes in Terminal mode the
        // active pane gets its keys back; in another mode nothing takes the
        // keyboard.
        ModesOutcome::MenuClosed(_mode) => {
            terminal::deliver(app, TerminalIntent::FocusActive)
        },
        ModesOutcome::PinnedSaveFailed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

#[cfg(test)]
mod tests {
    use otty_app_domain::mode::Mode;

    use super::*;
    use crate::testing::task_outputs::{
        focuses_nothing, runs_widget_operation,
    };
    use crate::testing::window::{app, open_shell_tab};

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

    /// A failed save of the pinned modes shows as a toast in the same
    /// update and counts as unseen on the bell.
    #[test]
    fn a_failed_pinned_save_shows_as_a_toast() {
        let mut app = app();

        let _none = deliver(
            &mut app,
            ModesIntent::PinnedSaved(Err(String::from("disk full"))),
        );

        let expected = otty_app_i18n::with_error(
            otty_app_i18n::Key::TplPinnedModesSaveFailed,
            "disk full",
        );
        let mut ui = iced_test::simulator(crate::view::view(&app));
        assert_eq!(app.notifications.unseen_count(), 1);
        assert!(ui.find(expected.as_str()).is_ok());
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

    /// Once the menu closes in Terminal mode, the active pane gets its keys
    /// back; with a shell tab in Containers mode, still nothing does.
    #[test]
    fn a_menu_closed_in_terminal_mode_gives_the_pane_its_keys() {
        let mut terminal = app();
        let _tab = open_shell_tab(&mut terminal);
        let _open = deliver(&mut terminal, ModesIntent::ToggleMenu);
        let mut containers = app();
        let _tab = open_shell_tab(&mut containers);
        let _select =
            deliver(&mut containers, ModesIntent::Select(Mode::Containers));
        let _open = deliver(&mut containers, ModesIntent::ToggleMenu);

        let in_terminal = deliver(&mut terminal, ModesIntent::CloseMenu);
        let in_containers = deliver(&mut containers, ModesIntent::CloseMenu);

        assert!(runs_widget_operation(in_terminal));
        assert!(!runs_widget_operation(in_containers));
    }

    /// Back in Terminal mode the active pane takes the keyboard.
    #[test]
    fn back_in_terminal_mode_the_active_pane_takes_the_keys() {
        let mut app = app();
        let _tab = open_shell_tab(&mut app);
        let _away = deliver(&mut app, ModesIntent::Select(Mode::Containers));

        let back = deliver(&mut app, ModesIntent::Select(Mode::Terminal));

        assert!(runs_widget_operation(back));
    }

    /// Containers mode looks for engines when it is entered, and talks to
    /// them only while it is active.
    #[test]
    fn containers_mode_talks_to_the_engines_only_while_it_is_active() {
        let mut app = app();

        let _to = deliver(&mut app, ModesIntent::Select(Mode::Containers));
        let entered = app.containers.is_active();
        let _back = deliver(&mut app, ModesIntent::Select(Mode::Terminal));

        assert!(entered);
        assert!(!app.containers.is_active());
    }
}
