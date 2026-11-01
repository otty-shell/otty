use super::types::{LanguageSetting, SettingsData, SettingsLoad};
use crate::theme::BuiltinTheme;

/// Intent events handled by the settings presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum SettingsIntent {
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
    /// The interface language selection changed.
    LanguageChanged(LanguageSetting),
    /// The shell text input changed.
    ShellChanged(String),
    /// The editor text input changed.
    EditorChanged(String),
    /// The equalize-panes checkbox was toggled.
    EqualizePanesToggled(bool),
    /// A theme was picked in the theme selector.
    ThemeSelected(BuiltinTheme),
}

/// Effect events produced by the settings reducer, routed outward.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEffect {
    /// Reload completed and produced a load payload.
    ReloadLoaded(SettingsLoad),
    /// Reload failed.
    ReloadFailed(String),
    /// Save failed.
    SaveFailed(String),
    /// Request the parent to apply the given settings, including the theme.
    ApplyTheme(SettingsData),
    /// Notify the parent that a save completed.
    SaveCompleted(SettingsData),
}

/// Settings event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    /// Intent event reduced by the settings widget.
    Intent(SettingsIntent),
    /// External effect orchestrated by app-level routing.
    Effect(SettingsEffect),
}
