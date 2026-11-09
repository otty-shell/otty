use super::state::SettingsState;
use crate::widgets::settings::types::SettingsData;

/// Read-only view model for the Settings popup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsViewModel<'a> {
    pub(super) draft: &'a SettingsData,
    pub(super) is_dirty: bool,
    pub(super) is_saving: bool,
}

impl<'a> SettingsViewModel<'a> {
    /// Build the view model from the settings state; `None` while the popup
    /// is closed.
    pub(super) fn from_state(state: &'a SettingsState) -> Option<Self> {
        if !state.is_open() {
            return None;
        }

        Some(Self {
            draft: state.draft(),
            is_dirty: state.is_dirty(),
            is_saving: state.is_saving(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_popup_has_no_view_model_and_an_open_one_has() {
        let mut state = SettingsState::default();
        let closed = SettingsViewModel::from_state(&state).is_none();

        let _ = state.open();

        assert!(closed);
        assert!(SettingsViewModel::from_state(&state).is_some());
    }
}
