use iced::window;

/// UI events emitted by chrome widget views.
#[derive(Debug, Clone)]
pub(crate) enum ChromeEvent {
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
