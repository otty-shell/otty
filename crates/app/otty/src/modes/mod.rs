//! The modes: the active mode, the pinned modes of the switcher and the All
//! modes menu. The side panels stay with the sidebar.

mod event;
mod model;
mod reducer;
mod state;
pub(crate) mod types;
pub(crate) mod view;

use otty_app_domain::mode::Mode;

pub(crate) use self::event::{ModesIntent, ModesOutcome};
pub(crate) use self::model::ModesViewModel;
use self::state::ModesState;

/// Modes widget: the active mode, the pinned modes and the All modes menu.
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

    /// Start in Terminal mode with `pinned` pinned in order. Entries the pin
    /// rules refuse (Terminal, repeats, entries past the limit) are left out
    /// and logged.
    pub(crate) fn with_pinned(pinned: &[Mode]) -> Self {
        let state = ModesState::with_pinned(pinned);

        if state.pinned().len() < pinned.len() {
            log::warn!(
                "modes.json: Terminal, repeated modes and modes past the pin \
                 limit skipped"
            );
        }

        Self { state }
    }

    /// Build the view model of the switcher and the All modes menu.
    pub(crate) fn vm(&self) -> ModesViewModel {
        ModesViewModel::from_state(&self.state)
    }
}

#[cfg(test)]
impl ModesWidget {
    /// Pinned modes in the order they were pinned; never Terminal.
    pub(crate) fn pinned(&self) -> &[Mode] {
        self.state.pinned()
    }
}

impl otty_app_slots::Feature for ModesWidget {
    type Intent = ModesIntent;
    type Outcome = ModesOutcome;
    type Ctx<'a> = ();

    fn reduce(&mut self, intent: ModesIntent, _ctx: ()) -> Vec<ModesOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}
