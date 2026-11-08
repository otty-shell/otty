mod errors;
pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
#[expect(
    clippy::disallowed_methods,
    reason = "this module saves the pinned modes"
)]
pub(crate) mod storage;
pub(crate) mod types;
pub(crate) mod view;

pub(crate) use event::{ModesEffect, ModesEvent, ModesIntent};
use iced::Task;
pub(crate) use model::ModesViewModel;
use state::ModesState;

use crate::domain::mode::Mode;

/// Modes widget: the active mode, the pinned modes and the All modes menu
/// (spec Decision 5). The side panels stay with the sidebar widget.
pub(crate) struct ModesWidget {
    state: ModesState,
}

impl ModesWidget {
    /// Mode shown in the left panel, main area, right panel and status bar.
    pub(crate) fn active(&self) -> Mode {
        self.state.active()
    }

    /// Return whether the All modes menu is open.
    pub(crate) fn is_menu_open(&self) -> bool {
        self.state.is_menu_open()
    }

    /// Start in Terminal mode with the pinned modes saved on disk.
    pub(crate) fn load() -> Self {
        Self {
            state: storage::load_initial_modes_state(),
        }
    }

    /// Reduce an intent into state changes and effect events.
    pub(crate) fn reduce(&mut self, intent: ModesIntent) -> Task<ModesEvent> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Build the view model of the switcher and the All modes menu.
    pub(crate) fn vm(&self) -> ModesViewModel {
        ModesViewModel::from_state(&self.state)
    }
}

#[cfg(test)]
impl ModesWidget {
    /// First start state, without reading `modes.json`.
    pub(crate) fn new() -> Self {
        Self {
            state: ModesState::default(),
        }
    }
}
