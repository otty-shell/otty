/// Commands processed by the chrome widget reducer.
#[derive(Debug, Clone)]
pub(crate) enum ChromeCommand {
    ToggleFullScreen,
    MinimizeWindow,
    CloseWindow,
    ToggleSidebarVisibility,
    StartWindowDrag,
}
