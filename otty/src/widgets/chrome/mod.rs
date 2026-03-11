pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod view;

use iced::Task;

pub(crate) use self::event::{ChromeEffect, ChromeEvent, ChromeIntent};
use self::model::ChromeViewModel;
use self::state::ChromeState;

/// Chrome widget managing window decorations and controls.
pub(crate) struct ChromeWidget {
    state: ChromeState,
}

impl ChromeWidget {
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

    /// Produce the chrome view model for rendering.
    pub(crate) fn vm(&self) -> ChromeViewModel {
        ChromeViewModel {
            is_fullscreen: self.state.is_fullscreen(),
        }
    }
}
