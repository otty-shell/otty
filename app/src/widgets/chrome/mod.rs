pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod view;

use iced::Task;

pub(crate) use self::event::{ChromeEffect, ChromeEvent, ChromeIntent};
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

    /// Reduce a chrome intent event into state updates and effects.
    pub(crate) fn reduce(&mut self, event: ChromeIntent) -> Task<ChromeEvent> {
        reducer::reduce(&mut self.state, event)
    }
}
