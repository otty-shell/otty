use super::model::SettingsData;
use super::state::SettingsPreset;
use super::storage::SettingsLoad;

/// UI and internal events handled by the settings feature reducer.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    Reload,
    ReloadLoaded(SettingsLoad),
    ReloadFailed(String),
    Save,
    SaveCompleted(SettingsData),
    SaveFailed(String),
    Reset,
    NodePressed { path: Vec<String> },
    NodeHovered { path: Option<Vec<String>> },
    ShellChanged(String),
    EditorChanged(String),
    PaletteChanged { index: usize, value: String },
    ApplyPreset(SettingsPreset),
}
