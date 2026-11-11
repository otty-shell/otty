use otty_app_domain::commands::AppCommand;

/// What the user does on the title bar.
#[derive(Debug, Clone)]
pub(crate) enum TitleBarIntent {
    /// Maximize the window to the free screen area, or restore it.
    ToggleMaximize,
    MinimizeWindow,
    CloseWindow,
    /// Show or hide the left panel.
    ToggleLeftPanel,
    /// Show or hide the files (right) panel.
    ToggleFilesPanel,
    /// Enter or leave focus mode.
    ToggleFocusMode,
    /// Open the Settings popup over the active mode.
    OpenSettings,
    /// The bell was pressed: open the notifications list.
    OpenNotifications,
    /// The title bar search was pressed: open the command palette.
    OpenCommandPalette,
    /// The left button went down on the empty title bar.
    PressTitleBar,
    /// The pointer moved, or left the title bar, while the button is held.
    MoveOnTitleBar,
    /// The left button went up on the title bar.
    ReleaseTitleBar,
}

/// What the title bar reducer reports: a header button that stands for an app
/// command, or an operation on the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TitleBarOutcome {
    /// A header button was pressed that does what this command does.
    Command(AppCommand),
    /// Maximize the window to the free screen area, or restore it.
    ToggleMaximize,
    /// Minimize the window.
    MinimizeWindow,
    /// Close the window.
    CloseWindow,
    /// The pointer moved with the button held on the title bar: the window
    /// manager moves the window from here.
    StartWindowDrag,
}
