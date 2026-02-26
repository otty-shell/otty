use super::model::{SettingsData, SettingsPreset};
use super::storage::SettingsLoad;

/// Internal commands dispatched to the settings reducer.
#[derive(Debug, Clone)]
pub(crate) enum SettingsCommand {
    /// Request a fresh load from disk.
    Reload,
    /// Disk load completed successfully.
    ReloadLoaded(SettingsLoad),
    /// Disk load failed.
    ReloadFailed(String),
    /// Request saving the current draft to disk.
    Save,
    /// Save completed; carries the normalized settings that were written.
    SaveCompleted(SettingsData),
    /// Save failed.
    SaveFailed(String),
    /// Discard draft edits and restore the baseline.
    Reset,
    /// A tree node was pressed.
    NodePressed { path: Vec<String> },
    /// A tree node was hovered (or unhovered when `path` is `None`).
    NodeHovered { path: Option<Vec<String>> },
    /// The shell text input changed.
    ShellChanged(String),
    /// The editor text input changed.
    EditorChanged(String),
    /// A palette color text input changed.
    PaletteChanged { index: usize, value: String },
    /// A theme preset was selected.
    ApplyPreset(SettingsPreset),
}
