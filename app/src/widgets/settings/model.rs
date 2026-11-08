use super::state::SettingsState;
use crate::widgets::settings::types::SettingsData;

/// Read-only view model for the settings form.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsViewModel<'a> {
    pub(super) draft: &'a SettingsData,
    pub(super) is_dirty: bool,
    pub(super) is_saving: bool,
}

impl<'a> SettingsViewModel<'a> {
    /// Build the view model from the settings state.
    pub(super) fn from_state(state: &'a SettingsState) -> Self {
        Self {
            draft: state.draft(),
            is_dirty: state.is_dirty(),
            is_saving: state.is_saving(),
        }
    }
}
