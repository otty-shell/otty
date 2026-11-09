mod errors;
pub(crate) mod event;
mod model;
mod reducer;
mod state;
#[expect(clippy::disallowed_methods, reason = "this module saves the settings")]
pub(crate) mod storage;
pub(crate) mod types;
pub(crate) mod view;

use event::{SettingsEvent, SettingsIntent};
use iced::Task;
use model::SettingsViewModel;
use state::SettingsState;
use types::SettingsData;

/// Settings widget: the Settings popup over every mode, where the user edits
/// the application settings (language, terminal shell and editor, theme) in
/// a draft and saves them.
pub(crate) struct SettingsWidget {
    state: SettingsState,
    /// Why the settings file could not be used at start, ready to show.
    load_problem: Option<String>,
}

impl SettingsWidget {
    /// Return whether the Settings popup is open.
    pub(crate) fn is_open(&self) -> bool {
        self.state.is_open()
    }

    /// Return the saved terminal editor command. An edit in the popup
    /// counts only once it is saved.
    pub(crate) fn terminal_editor(&self) -> &str {
        self.state.baseline().terminal_editor()
    }

    /// Return the saved settings the app runs with. Edits in the open popup
    /// are not in them until Save.
    pub(crate) fn saved(&self) -> &SettingsData {
        self.state.baseline()
    }

    /// Why the settings file could not be used at start, in the user's
    /// language; `None` when it was read or is missing (the first start).
    pub(crate) fn load_problem(&self) -> Option<&str> {
        self.load_problem.as_deref()
    }

    /// Create a widget by loading persisted state from disk.
    pub(crate) fn load() -> Self {
        let (state, load_problem) = storage::load_initial_settings_state();

        Self {
            state,
            load_problem,
        }
    }

    /// Reduce an intent event into state updates and effects.
    pub(crate) fn reduce(
        &mut self,
        intent: SettingsIntent,
    ) -> Task<SettingsEvent> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Return the view model of the popup; `None` while it is closed.
    pub(crate) fn vm(&self) -> Option<SettingsViewModel<'_>> {
        SettingsViewModel::from_state(&self.state)
    }
}

#[cfg(test)]
impl SettingsWidget {
    /// Return whether the popup holds edits that were not saved.
    pub(crate) fn has_unsaved_edits(&self) -> bool {
        self.state.is_dirty()
    }
}
