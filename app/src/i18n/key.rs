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
    // New session menu.
    /// "New terminal" entry in the New session menu.
    MenuNewTerminal,
    /// "Split current pane right" entry in the New session menu.
    MenuSplitCurrentPane,
    /// "New launch..." entry in the New session menu.
    MenuNewLaunch,
    /// Tooltip of the New session menu chevron.
    NewSessionMenu,

    // Placeholders shown when tab state is missing.
    /// Shown when a terminal tab has no backing state.
    TerminalTabNotInitialized,
    /// Shown when a quick launch wizard tab has no editor state.
    QuickLaunchEditorNotInitialized,
    /// Shown when a quick launch error tab has no payload.
    QuickLaunchErrorPayloadMissing,
    /// Shown when the terminal widget cannot be rendered.
    TerminalUnavailable,
    /// Banner on a kept-open pane whose process ended without an exit code.
    ProcessTerminated,

    // Tab titles.
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
    /// Split the pane into a top and a bottom pane.
    SplitDown,
    /// Split the pane into a left and a right pane.
    SplitRight,
    /// Close the pane.
    ClosePane,
    /// Name of this computer when its host name cannot be read.
    ThisComputer,
    /// Tooltip of the title bar full screen button.
    WindowToggleFullscreen,
    /// Tooltip of the title bar minimize button.
    WindowMinimize,
    /// Tooltip of the title bar button that shows or hides the files panel.
    WindowFilesPanel,
    /// Tooltip of the title bar button that shows or hides the left panel.
    WindowLeftPanel,
    /// Tooltip of the title bar focus mode button.
    WindowFocusMode,
    /// Focus mode note: says the panels are hidden.
    FocusNoteHidden,
    /// Focus mode note: text after the shortcut.
    FocusNoteShow,
    /// Tooltip of the title bar button that closes the window.
    WindowClose,

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

    // Sessions section of the left panel.
    /// Title of the Sessions section of the left panel.
    SessionsPanelTitle,
    /// Tooltip of the Sessions header button that opens a terminal.
    NewSession,
    /// Muted note after this computer's group name.
    ThisComputerNote,
    /// Muted note after an SSH host's group name.
    SshMachineNote,
    /// State word on the Settings row.
    SessionStateSettings,
    /// State word on a launch editor row.
    SessionStateEditor,
    /// State word on a launch error row.
    SessionStateFailed,
    /// Hint in the Sessions section when no session is open.
    NoOpenSessions,

    // Empty state shown in the main area when no session is open.
    /// Title of the empty card.
    EmptyTitle,
    /// Description under the title of the empty card.
    EmptyDescription,
    /// Label of the empty card tile that opens a terminal.
    EmptyNewTerminal,
    /// Label of the empty card tile that creates a launch.
    EmptyNewLaunch,

    // Quick launch sidebar panel.
    /// Title of the Launches section of the left panel.
    LaunchesPanelTitle,
    /// Muted word on a launch row while the launch is running.
    LaunchRunning,

    // Explorer sidebar.
    /// Shown when the explorer has no active folder.
    NoActiveFolder,
    /// Title of the right panel files section.
    FilesTitle,
    /// Explorer line naming the followed shell. Placeholder: `{shell}`.
    TplFilesFollows,

    // Status bar.
    /// Status bar text for exactly one pane.
    StatusPaneOne,
    /// Status bar pane count. Placeholder: `{count}`.
    TplStatusPanes,
    /// Status bar text for exactly one session.
    StatusSessionOne,
    /// Status bar session count. Placeholder: `{count}`.
    TplStatusSessions,
    // Command palette and its title bar trigger.
    /// Title bar search text while no tab is open.
    TitleSearchNoSessions,
    /// Text in the empty palette input.
    PalettePlaceholder,
    /// Line shown when no palette entry matches the query.
    PaletteNoMatches,
    /// Footer hint after the ↑↓ keys.
    PaletteHintMove,
    /// Footer hint after the Enter key.
    PaletteHintOpen,
    /// Group label of the app actions.
    PaletteGroupActions,
    /// Action that opens a terminal tab.
    PaletteNewTerminal,
    /// Action that opens the editor for a new launch.
    PaletteNewLaunch,
    /// Action that starts a new launch folder.
    PaletteNewFolder,
    /// Action that splits the focused pane.
    PaletteSplitPane,
    /// Action that closes the active tab.
    PaletteCloseSession,
    /// Action that shows or hides the left panel.
    PaletteToggleLeftPanel,
    /// Action that shows or hides the files panel.
    PaletteToggleFilesPanel,

    // Quick launch wizard.
    /// Title of the form that creates a launch.
    WizardTitleNew,
    /// Title of the form that edits a launch.
    WizardTitleEdit,
    /// Muted line under the launch form title.
    WizardSubtitle,
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
    /// Label of the choice for what a tab does once its command exits.
    FieldAfterExit,
    /// Muted hint under the exit choice.
    ExitBehaviorHint,
    /// Exit choice: close the tab.
    ExitBehaviorClose,
    /// Exit choice: close the tab only when the command succeeds.
    ExitBehaviorCloseOnSuccess,
    /// Exit choice: keep the tab open.
    ExitBehaviorKeepOpen,
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
    /// Connect with the secret typed in the SSH secret dialog.
    ButtonConnect,

    // SSH secret dialog.
    /// Dialog title when a private key needs its passphrase.
    SecretPromptPassphraseTitle,
    /// Dialog title when the server needs a password.
    SecretPromptPasswordTitle,
    /// Shown when the typed passphrase did not unlock the key.
    SecretPromptWrongPassphrase,
    /// Shown when the server rejected the typed password.
    SecretPromptPasswordRejected,
    /// Field label for the key passphrase.
    SecretPromptPassphraseLabel,
    /// Field label for the account password.
    SecretPromptPasswordLabel,

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
    /// Equalize panes setting label.
    OptionEqualizePanes,
    /// Line under the equalize panes setting.
    DescEqualizePanes,
    /// Theme selector field label.
    FieldTheme,
    /// Line under the theme setting.
    DescTheme,
    /// Line under the language setting.
    DescLanguage,
    /// Line under the shell setting.
    DescShell,
    /// Line under the default editor setting.
    DescDefaultEditor,
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
    /// SSH validation: port is not positive.
    ErrSshPortPositive,
    /// Wizard save target folder is missing.
    ErrMissingTargetFolder,
    /// Wizard save parent folder is missing.
    ErrMissingParentFolder,
    /// Wizard edit target no longer exists.
    ErrCommandNoLongerExists,

    // Templates. These carry `{placeholder}` markers substituted at call time.
    /// Tab title for editing an existing quick launch. Placeholder: `{title}`.
    TplEditTabTitle,
    /// Error tab title for a failed launch. Placeholder: `{title}`.
    TplLaunchFailedTitle,
    /// Error tab body for a failed launch. Placeholders: `{command}`, `{error}`.
    TplLaunchFailedBody,
    /// Message for a terminal that failed to start. Placeholder: `{error}`.
    TplTerminalInitFailed,
    /// SSH connection failure. Placeholder: `{error}`.
    TplSshConnectionFailed,
    /// Missing working directory. Placeholder: `{path}`.
    TplWorkingDirectoryNotFound,
    /// Working path is not a directory. Placeholder: `{path}`.
    TplWorkingDirectoryNotDirectory,
    /// Missing SSH identity file. Placeholder: `{path}`.
    TplIdentityFileNotFound,
    /// SSH identity path is not a file. Placeholder: `{path}`.
    TplIdentityFileNotFile,
    /// SSH secret dialog title for a key passphrase. Placeholder: `{path}`.
    TplSecretPromptPassphraseTitle,
    /// SSH secret dialog title for a password. Placeholder: `{target}`.
    TplSecretPromptPasswordTitle,
    /// Muted line under the passphrase title. Placeholders: `{launch}`,
    /// `{account}`.
    TplSecretPromptPassphraseHint,
    /// Muted line under the password title. Placeholders: `{launch}`,
    /// `{account}`.
    TplSecretPromptPasswordHint,
    /// Program is absent from `PATH`. Placeholder: `{program}`.
    TplProgramNotFoundInPath,
    /// Explicit program path is missing. Placeholder: `{program}`.
    TplProgramNotFound,
    /// Program path is a directory. Placeholder: `{program}`.
    TplProgramIsDirectory,
    /// Program path is not executable. Placeholder: `{program}`.
    TplProgramNotExecutable,
    /// Banner on a kept-open pane whose process exited. Placeholder: `{code}`.
    TplProcessExited,
}

impl Key {
    /// Every key in the catalogs, used by tests to verify full coverage.
    #[cfg(test)]
    pub(super) const ALL: [Self; 157] = [
        Self::MenuNewTerminal,
        Self::MenuSplitCurrentPane,
        Self::MenuNewLaunch,
        Self::NewSessionMenu,
        Self::TerminalTabNotInitialized,
        Self::QuickLaunchEditorNotInitialized,
        Self::QuickLaunchErrorPayloadMissing,
        Self::TerminalUnavailable,
        Self::ProcessTerminated,
        Self::SettingsTabTitle,
        Self::QuickLaunchCreateTabTitle,
        Self::CopySelection,
        Self::Paste,
        Self::CopyContent,
        Self::CopyPrompt,
        Self::CopyCommand,
        Self::SplitDown,
        Self::SplitRight,
        Self::ClosePane,
        Self::ThisComputer,
        Self::WindowToggleFullscreen,
        Self::WindowMinimize,
        Self::WindowFilesPanel,
        Self::WindowLeftPanel,
        Self::WindowFocusMode,
        Self::FocusNoteHidden,
        Self::FocusNoteShow,
        Self::WindowClose,
        Self::Kill,
        Self::Edit,
        Self::Rename,
        Self::Duplicate,
        Self::Remove,
        Self::CreateFolder,
        Self::CreateQuickLaunch,
        Self::Delete,
        Self::CreateLaunch,
        Self::SessionsPanelTitle,
        Self::NewSession,
        Self::ThisComputerNote,
        Self::SshMachineNote,
        Self::SessionStateSettings,
        Self::SessionStateEditor,
        Self::SessionStateFailed,
        Self::NoOpenSessions,
        Self::EmptyTitle,
        Self::EmptyDescription,
        Self::EmptyNewTerminal,
        Self::EmptyNewLaunch,
        Self::LaunchesPanelTitle,
        Self::LaunchRunning,
        Self::NoActiveFolder,
        Self::FilesTitle,
        Self::TplFilesFollows,
        Self::StatusPaneOne,
        Self::TplStatusPanes,
        Self::StatusSessionOne,
        Self::TplStatusSessions,
        Self::TitleSearchNoSessions,
        Self::PalettePlaceholder,
        Self::PaletteNoMatches,
        Self::PaletteHintMove,
        Self::PaletteHintOpen,
        Self::PaletteGroupActions,
        Self::PaletteNewTerminal,
        Self::PaletteNewLaunch,
        Self::PaletteNewFolder,
        Self::PaletteSplitPane,
        Self::PaletteCloseSession,
        Self::PaletteToggleLeftPanel,
        Self::PaletteToggleFilesPanel,
        Self::WizardTitleNew,
        Self::WizardTitleEdit,
        Self::WizardSubtitle,
        Self::ExitBehaviorHint,
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
        Self::FieldAfterExit,
        Self::ExitBehaviorClose,
        Self::ExitBehaviorCloseOnSuccess,
        Self::ExitBehaviorKeepOpen,
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
        Self::ButtonConnect,
        Self::SecretPromptPassphraseTitle,
        Self::SecretPromptPasswordTitle,
        Self::SecretPromptWrongPassphrase,
        Self::SecretPromptPasswordRejected,
        Self::SecretPromptPassphraseLabel,
        Self::SecretPromptPasswordLabel,
        Self::SectionGeneral,
        Self::SectionTerminal,
        Self::SectionAppearance,
        Self::FieldLanguage,
        Self::FieldShell,
        Self::FieldDefaultEditor,
        Self::OptionEqualizePanes,
        Self::DescEqualizePanes,
        Self::FieldTheme,
        Self::DescTheme,
        Self::DescLanguage,
        Self::DescShell,
        Self::DescDefaultEditor,
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
        Self::ErrSshPortPositive,
        Self::ErrMissingTargetFolder,
        Self::ErrMissingParentFolder,
        Self::ErrCommandNoLongerExists,
        Self::TplEditTabTitle,
        Self::TplLaunchFailedTitle,
        Self::TplLaunchFailedBody,
        Self::TplTerminalInitFailed,
        Self::TplSshConnectionFailed,
        Self::TplWorkingDirectoryNotFound,
        Self::TplWorkingDirectoryNotDirectory,
        Self::TplIdentityFileNotFound,
        Self::TplIdentityFileNotFile,
        Self::TplSecretPromptPassphraseTitle,
        Self::TplSecretPromptPasswordTitle,
        Self::TplSecretPromptPassphraseHint,
        Self::TplSecretPromptPasswordHint,
        Self::TplProgramNotFoundInPath,
        Self::TplProgramNotFound,
        Self::TplProgramIsDirectory,
        Self::TplProgramNotExecutable,
        Self::TplProcessExited,
    ];
}
