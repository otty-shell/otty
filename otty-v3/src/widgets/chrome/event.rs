use iced::window;

/// UI events emitted by chrome widget views.
#[derive(Debug, Clone)]
pub(crate) enum ChromeUiEvent {
    ToggleFullScreen,
    MinimizeWindow,
    CloseWindow,
    ToggleSidebarVisibility,
    StartWindowDrag,
}

/// Effect events produced by the chrome reducer.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEffect {
    FullScreenToggled { mode: window::Mode },
    MinimizeWindow,
    CloseWindow,
    ToggleSidebarVisibility,
    StartWindowDrag,
}

/// Chrome event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEvent {
    /// UI/internal event reduced by the chrome widget.
    Ui(ChromeUiEvent),
    /// External effect orchestrated by app-level routing.
    Effect(ChromeEffect),
}
