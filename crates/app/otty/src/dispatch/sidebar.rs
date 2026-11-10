use iced::Task;
use otty_app_slots::Feature;

use crate::app::App;
use crate::message::AppEvent;
use crate::sidebar::{SidebarIntent, SidebarOutcome};

/// Give a sidebar intent to the widget and read what it reports.
pub(crate) fn deliver(app: &mut App, intent: SidebarIntent) -> Task<AppEvent> {
    let outcomes = app.sidebar.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes.into_iter().map(read).collect();

    Task::batch(tasks)
}

fn read(outcome: SidebarOutcome) -> Task<AppEvent> {
    match outcome {
        SidebarOutcome::HeaderMenuOpened => super::take_keyboard(),
        // The terminal feature (issue 11) resizes its pane grids to the new
        // main area.
        SidebarOutcome::MainAreaResized => Task::none(),
        // The quick launch feature (issue 12) stops the hover effects of
        // the Launches tree until the drag settles.
        SidebarOutcome::PanelEdgeDragged => Task::none(),
        // The terminal feature (issue 11) gives the active pane its keys
        // back in Terminal mode; in Containers mode nothing gets them.
        SidebarOutcome::HeaderMenuClosed => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidebar::types::HeaderMenu;
    use crate::testing::task_outputs::focuses_nothing;
    use crate::testing::window::app;

    /// The header menu takes the keyboard from the focused terminal, so
    /// typed keys do not reach its shell and Esc reaches the frame.
    #[test]
    fn an_opened_header_menu_takes_the_keyboard_focus() {
        let mut app = app();

        let task = deliver(
            &mut app,
            SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions),
        );

        assert!(app.sidebar.has_header_menu_open());
        assert!(focuses_nothing(task));
    }
}
