use crate::widgets::settings::types::SettingsData;

/// Read-only view model for the settings form.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsViewModel<'a> {
    pub(super) draft: &'a SettingsData,
    pub(super) is_dirty: bool,
    pub(super) is_saving: bool,
}
