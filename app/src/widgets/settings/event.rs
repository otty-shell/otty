use super::types::{LanguageSetting, SettingsData, SettingsLoad};
use crate::theme::BuiltinTheme;

/// Intent events handled by the Settings popup.
#[derive(Debug, Clone)]
pub(crate) enum SettingsIntent {
    /// Open the popup and read the saved settings again. Nothing happens
    /// while it is open.
    Open,
    /// Close the popup and drop the edits that were not saved: Esc, the
    /// close cross, or Esc in a field of the popup.
    Close,
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
    /// Read the settings from disk; the result comes back as an intent.
    LoadRequested,
    /// Write these settings to disk; the result comes back as an intent.
    SaveRequested(SettingsData),
    /// Request the parent to apply the given settings, including the theme.
    ApplyTheme(SettingsData),
    /// The popup closed; the keyboard goes back to the active pane.
    Closed,
}

/// Settings event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    /// Intent event reduced by the settings widget.
    Intent(SettingsIntent),
    /// External effect orchestrated by app-level routing.
    Effect(SettingsEffect),
}
