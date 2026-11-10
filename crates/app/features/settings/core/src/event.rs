use otty_app_theme::theme::BuiltinTheme;

use crate::problem::SettingsProblem;
use crate::types::{LanguageSetting, SettingsData, SettingsLoad};

/// What the Settings dialog is asked to do.
#[derive(Debug, Clone)]
pub enum SettingsIntent {
    /// Open the dialog and read the saved settings again. Nothing happens
    /// while it is open.
    Open,
    /// Close the dialog and drop the edits that were not saved: Esc, the
    /// close cross, a press on the dimmed area around it, or Esc in a field
    /// of the dialog.
    Close,
    /// The settings file was read; the binary's answer to
    /// [`SettingsOutcome::ReadSaved`].
    ReloadLoaded(SettingsLoad),
    /// The settings file could not be read; carries the error.
    ReloadFailed(String),
    /// Save the current draft.
    Save,
    /// The save finished; carries the settings that were written.
    SaveCompleted(SettingsData),
    /// The save failed; carries the error.
    SaveFailed(String),
    /// Drop the draft edits and show the saved settings again.
    Reset,
    /// The interface language selection changed.
    LanguageChanged(LanguageSetting),
    /// The shell text input changed.
    ShellChanged(String),
    /// The editor text input changed.
    EditorChanged(String),
    /// The equalize-panes switch was toggled.
    EqualizePanesToggled(bool),
    /// A theme was picked in the theme selector.
    ThemeSelected(BuiltinTheme),
}

/// What the Settings dialog reports back to the frame.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsOutcome {
    /// The dialog opened: it takes the keyboard from every widget, the
    /// terminal too, so typed keys reach no shell and Esc reaches the frame.
    Opened,
    /// Read the saved settings; answer [`SettingsIntent::ReloadLoaded`] or
    /// [`SettingsIntent::ReloadFailed`].
    ReadSaved,
    /// Write these settings; answer [`SettingsIntent::SaveCompleted`] or
    /// [`SettingsIntent::SaveFailed`].
    WriteSaved(SettingsData),
    /// These settings were saved or read again: the window and new
    /// terminals take their language, theme and shell.
    Applied(SettingsData),
    /// The dialog closed: in Terminal mode the active pane gets the keyboard
    /// back.
    Closed,
    /// Tell the user about a failure.
    Failed(SettingsProblem),
}
