use serde::Deserialize;

/// Identifier for a user-facing string in the translation catalogs.
///
/// Variants map to `snake_case` keys in the locale JSON files under
/// `locales/`. Every variant must be present in every catalog; the
/// `catalogs_cover_every_key` test enforces that, and an unknown key in a JSON
/// file fails deserialization at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Key {
    // Sidebar add menu.
    /// "Create tab" entry in the sidebar add menu.
    MenuCreateTab,
    /// "Create folder" entry in the sidebar add menu.
    MenuCreateFolder,
    /// "Create quick launch" entry in the sidebar add menu.
    MenuCreateQuickLaunch,

    // Placeholders shown when tab state is missing.
    /// Shown when a terminal tab has no backing state.
    TerminalTabNotInitialized,
    /// Shown when a quick launch wizard tab has no editor state.
    QuickLaunchEditorNotInitialized,
    /// Shown when a quick launch error tab has no payload.
    QuickLaunchErrorPayloadMissing,
    /// Shown when the terminal widget cannot be rendered.
    TerminalUnavailable,

    // Tab bar and tab content.
    /// Shown when no tab is open.
    NoTabs,
    /// Placeholder body for tabs without dedicated content.
    TabContentPlaceholder,
    /// Title of the settings tab.
    SettingsTabTitle,
    /// Title of the quick launch creation wizard tab.
    QuickLaunchCreateTabTitle,

    // Terminal pane context menu.
    /// Copy the current text selection.
    CopySelection,
    /// Paste clipboard contents into the terminal.
    Paste,
    /// Copy the selected block's output.
    CopyContent,
    /// Copy the selected block's prompt.
    CopyPrompt,
    /// Copy the selected block's command line.
    CopyCommand,
    /// Split the pane along the horizontal axis.
    SplitHorizontally,
    /// Split the pane along the vertical axis.
    SplitVertically,
    /// Close the pane.
    ClosePane,

    // Quick launch context menu.
    /// Terminate a running quick launch.
    Kill,
    /// Edit the selected quick launch.
    Edit,
    /// Rename the selected entry.
    Rename,
    /// Duplicate the selected quick launch.
    Duplicate,
    /// Remove the selected quick launch.
    Remove,
    /// Create a folder in the quick launch tree.
    CreateFolder,
    /// Create a quick launch entry.
    CreateQuickLaunch,
    /// Delete the selected folder.
    Delete,
    /// Create a launch entry inside a folder.
    CreateLaunch,

    // Quick launch sidebar panel.
    /// Header of the quick launch sidebar panel.
    QuickLaunchPanelTitle,

    // Explorer sidebar.
    /// Shown when the explorer has no active folder.
    NoActiveFolder,

    // Quick launch wizard.
    /// Wizard section header.
    WizardHeader,
    /// Entry title field label.
    FieldTitle,
    /// Command type field label.
    FieldType,
    /// Custom command type option.
    TypeCustom,
    /// SSH command type option.
    TypeSsh,
    /// Custom command section header.
    SectionCustomCommand,
    /// Program path field label.
    FieldProgram,
    /// Program arguments field label.
    FieldArguments,
    /// Working directory field label.
    FieldWorkdir,
    /// SSH connection section header.
    SectionSshConnection,
    /// SSH host field label.
    FieldHost,
    /// SSH port field label.
    FieldPort,
    /// SSH user field label.
    FieldUser,
    /// SSH identity file field label.
    FieldIdentityFile,
    /// Additional SSH argument field label.
    FieldExtraArgs,
    /// Environment variable list label.
    FieldEnvironment,
    /// Environment variable name placeholder.
    EnvKeyPlaceholder,
    /// Environment variable value placeholder.
    EnvValuePlaceholder,
    /// Shown when the custom editor state is inconsistent.
    InvalidCustomEditorState,
    /// Shown when the SSH editor state is inconsistent.
    InvalidSshEditorState,

    // Shared buttons.
    /// Persist pending changes.
    ButtonSave,
    /// Discard pending changes.
    ButtonCancel,
    /// Remove a list entry.
    ButtonRemove,
    /// Append a list entry.
    ButtonAdd,
    /// Append an environment variable entry.
    ButtonAddEnv,
    /// Restore settings to the persisted baseline.
    ButtonReset,

    // Settings sections and fields.
    /// General settings section.
    SectionGeneral,
    /// Terminal settings section.
    SectionTerminal,
    /// Appearance settings section.
    SectionAppearance,
    /// Interface language field label.
    FieldLanguage,
    /// Shell command field label.
    FieldShell,
    /// Default editor field label.
    FieldDefaultEditor,
    /// Theme preset field label.
    FieldPreset,
    /// Placeholder shown when no theme preset matches the palette.
    PresetPlaceholderCustom,
    /// Language option that follows the operating system locale.
    LanguageSystem,

    // Quick launch errors.
    /// Prefix for I/O error messages.
    ErrIoPrefix,
    /// Prefix for JSON error messages.
    ErrJsonPrefix,
    /// Rejected because the title was blank.
    ErrTitleEmpty,
    /// Rejected because a sibling already uses the title.
    ErrTitleDuplicate,
    /// Wizard validation: title missing.
    ErrTitleRequired,
    /// Wizard validation: program missing.
    ErrProgramRequired,
    /// Wizard validation: host missing.
    ErrHostRequired,
    /// Wizard validation: port is not numeric.
    ErrInvalidPort,
    /// Wizard validation: custom draft absent.
    ErrMissingCustomDraft,
    /// Wizard validation: SSH draft absent.
    ErrMissingSshDraft,

    // Templates. These carry `{placeholder}` markers substituted at call time.
    /// Tab title for editing an existing quick launch. Placeholder: `{title}`.
    TplEditTabTitle,
    /// Error tab title for a failed launch. Placeholder: `{title}`.
    TplLaunchFailedTitle,
    /// Error tab body for a failed launch. Placeholders: `{command}`, `{error}`.
    TplLaunchFailedBody,
    /// Message for a terminal that failed to start. Placeholder: `{error}`.
    TplTerminalInitFailed,
    /// Label for a palette color with no name. Placeholder: `{index}`.
    TplPaletteFallbackLabel,
}

impl Key {
    /// Every key in the catalogs, used by tests to verify full coverage.
    #[cfg(test)]
    pub(super) const ALL: [Self; 80] = [
        Self::MenuCreateTab,
        Self::MenuCreateFolder,
        Self::MenuCreateQuickLaunch,
        Self::TerminalTabNotInitialized,
        Self::QuickLaunchEditorNotInitialized,
        Self::QuickLaunchErrorPayloadMissing,
        Self::TerminalUnavailable,
        Self::NoTabs,
        Self::TabContentPlaceholder,
        Self::SettingsTabTitle,
        Self::QuickLaunchCreateTabTitle,
        Self::CopySelection,
        Self::Paste,
        Self::CopyContent,
        Self::CopyPrompt,
        Self::CopyCommand,
        Self::SplitHorizontally,
        Self::SplitVertically,
        Self::ClosePane,
        Self::Kill,
        Self::Edit,
        Self::Rename,
        Self::Duplicate,
        Self::Remove,
        Self::CreateFolder,
        Self::CreateQuickLaunch,
        Self::Delete,
        Self::CreateLaunch,
        Self::QuickLaunchPanelTitle,
        Self::NoActiveFolder,
        Self::WizardHeader,
        Self::FieldTitle,
        Self::FieldType,
        Self::TypeCustom,
        Self::TypeSsh,
        Self::SectionCustomCommand,
        Self::FieldProgram,
        Self::FieldArguments,
        Self::FieldWorkdir,
        Self::SectionSshConnection,
        Self::FieldHost,
        Self::FieldPort,
        Self::FieldUser,
        Self::FieldIdentityFile,
        Self::FieldExtraArgs,
        Self::FieldEnvironment,
        Self::EnvKeyPlaceholder,
        Self::EnvValuePlaceholder,
        Self::InvalidCustomEditorState,
        Self::InvalidSshEditorState,
        Self::ButtonSave,
        Self::ButtonCancel,
        Self::ButtonRemove,
        Self::ButtonAdd,
        Self::ButtonAddEnv,
        Self::ButtonReset,
        Self::SectionGeneral,
        Self::SectionTerminal,
        Self::SectionAppearance,
        Self::FieldLanguage,
        Self::FieldShell,
        Self::FieldDefaultEditor,
        Self::FieldPreset,
        Self::PresetPlaceholderCustom,
        Self::LanguageSystem,
        Self::ErrIoPrefix,
        Self::ErrJsonPrefix,
        Self::ErrTitleEmpty,
        Self::ErrTitleDuplicate,
        Self::ErrTitleRequired,
        Self::ErrProgramRequired,
        Self::ErrHostRequired,
        Self::ErrInvalidPort,
        Self::ErrMissingCustomDraft,
        Self::ErrMissingSshDraft,
        Self::TplEditTabTitle,
        Self::TplLaunchFailedTitle,
        Self::TplLaunchFailedBody,
        Self::TplTerminalInitFailed,
        Self::TplPaletteFallbackLabel,
    ];
}
