use crate::widgets::settings::types::{
    SettingsData, SettingsNode, SettingsPreset, SettingsSection,
};

/// Read-only view model for the settings form.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SettingsViewModel<'a> {
    pub(super) draft: &'a SettingsData,
    pub(super) palette_inputs: &'a [String],
    pub(super) selected_preset: Option<SettingsPreset>,
    pub(super) tree: &'a [SettingsNode],
    pub(super) selected_section: SettingsSection,
    pub(super) selected_path: &'a Vec<String>,
    pub(super) hovered_path: Option<&'a Vec<String>>,
    pub(super) is_dirty: bool,
}
