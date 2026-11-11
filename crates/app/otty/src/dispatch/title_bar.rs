use iced::{Task, window};

use super::{Delivery, command};
use crate::app::App;
use crate::frame::title_bar::{TitleBar, TitleBarIntent, TitleBarOutcome};
use crate::message::AppEvent;

/// The title bar reads what it reports: a header button runs its command
/// now, a window button becomes a window operation.
impl Delivery for TitleBarIntent {
    type Component = TitleBar;

    fn parts(app: &mut App) -> (&mut TitleBar, ()) {
        (&mut app.title_bar, ())
    }

    fn read(app: &mut App, outcome: TitleBarOutcome) -> Task<AppEvent> {
        match outcome {
            TitleBarOutcome::Command(asked) => command::run(app, asked),
            TitleBarOutcome::ToggleMaximize => {
                window::latest().and_then(window::toggle_maximize)
            },
            TitleBarOutcome::MinimizeWindow => {
                window::latest().and_then(|id| window::minimize(id, true))
            },
            TitleBarOutcome::CloseWindow => {
                window::latest().and_then(window::close)
            },
            TitleBarOutcome::StartWindowDrag => {
                window::latest().and_then(window::drag)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::deliver;
    use crate::testing::window::app;

    /// A header button runs its command in the same update: no message goes
    /// through iced in between.
    #[test]
    fn a_panel_button_toggles_its_panel_in_the_same_update() {
        let mut app = app();
        let before = app.sidebar.vm();

        let _left = deliver(&mut app, TitleBarIntent::ToggleLeftPanel);
        let _files = deliver(&mut app, TitleBarIntent::ToggleFilesPanel);
        let _focus = deliver(&mut app, TitleBarIntent::ToggleFocusMode);

        let after = app.sidebar.vm();
        assert_ne!(after.is_left_open, before.is_left_open);
        assert_ne!(after.is_right_open, before.is_right_open);
        assert!(after.is_focus_mode);
    }
}
