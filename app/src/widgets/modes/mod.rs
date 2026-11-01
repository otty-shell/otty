mod errors;
pub(crate) mod event;
pub(crate) mod model;
mod reducer;
mod state;
pub(crate) mod storage;
pub(crate) mod types;

pub(crate) use event::{ModesEffect, ModesEvent, ModesIntent};
use iced::Task;
pub(crate) use model::ModesViewModel;
use state::ModesState;

/// Modes widget: the active mode, the pinned modes and the All modes menu
/// (spec Decision 5). The side panels stay with the sidebar widget.
pub(crate) struct ModesWidget {
    state: ModesState,
}

impl ModesWidget {
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
