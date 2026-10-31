use iced::window;

/// Intent events handled by chrome widget views.
#[derive(Debug, Clone)]
pub(crate) enum ChromeIntent {
    ToggleFullScreen,
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
    StartWindowDrag,
}

/// Effect events produced by the chrome reducer.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEffect {
    FullScreenToggled { mode: window::Mode },
    MinimizeWindow,
    CloseWindow,
    ToggleLeftPanel,
    ToggleFilesPanel,
    ToggleFocusMode,
    OpenSettings,
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
