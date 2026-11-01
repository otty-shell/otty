/// Intent events handled by chrome widget views.
#[derive(Debug, Clone)]
pub(crate) enum ChromeIntent {
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
    /// Open the Settings tab.
    OpenSettings,
    /// The title bar search was pressed: open the command palette.
    OpenCommandPalette,
    /// The left button went down on the empty title bar.
    PressTitleBar,
    /// The pointer moved, or left the title bar, while the button is held.
    MoveOnTitleBar,
    /// The left button went up on the title bar.
    ReleaseTitleBar,
}

/// Effect events produced by the chrome reducer.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEffect {
    ToggleMaximize,
    MinimizeWindow,
    CloseWindow,
    ToggleLeftPanel,
    ToggleFilesPanel,
    ToggleFocusMode,
    OpenSettings,
    OpenCommandPalette,
    StartWindowDrag,
}

/// Chrome event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEvent {
    /// Intent event reduced by the chrome widget.
    Intent(ChromeIntent),
    /// External effect orchestrated by app-level routing.
    Effect(ChromeEffect),
}
