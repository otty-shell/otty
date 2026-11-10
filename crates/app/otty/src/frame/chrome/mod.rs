//! The chrome: the title bar with the window buttons, the panel toggles,
//! the search button, the bell and Settings, and the status bar view.

mod event;
mod model;
mod reducer;
mod state;
pub(crate) mod view;

pub(crate) use self::event::{ChromeIntent, ChromeOutcome};
pub(crate) use self::model::{StatusBarInput, StatusBarViewModel};
use self::state::ChromeState;

/// Chrome widget managing window decorations and controls.
pub(crate) struct ChromeWidget {
    state: ChromeState,
}

impl ChromeWidget {
    /// Return whether the left button is held on the empty title bar.
    pub(crate) fn is_title_bar_pressed(&self) -> bool {
        self.state.is_title_bar_pressed()
    }

    /// Create the chrome widget with default state.
    pub(crate) fn new() -> Self {
        Self {
            state: ChromeState::default(),
        }
    }
}

impl otty_app_slots::Component for ChromeWidget {
    type Intent = ChromeIntent;
    type Outcome = ChromeOutcome;
    type Ctx<'a> = ();

    fn reduce(&mut self, intent: ChromeIntent, _ctx: ()) -> Vec<ChromeOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}
