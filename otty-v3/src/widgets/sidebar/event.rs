/// UI events emitted by the sidebar presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    SelectTerminal,
    SelectExplorer,
    ToggleWorkspace,
    OpenSettings,
    AddMenuOpen,
    AddMenuDismiss,
    AddMenuCreateTab,
    AddMenuCreateCommand,
    AddMenuCreateFolder,
    WorkspaceCursorMoved { position: iced::Point },
    ToggleVisibility,
    PaneGridCursorMoved { position: iced::Point },
    Resized(iced::widget::pane_grid::ResizeEvent),
    DismissAddMenu,
}

/// Effect events produced by the sidebar reducer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEffect {
    SyncTerminalGridSizes,
    OpenSettingsTab,
    OpenTerminalTab,
    QuickLaunchHeaderCreateCommand,
    QuickLaunchHeaderCreateFolder,
    QuickLaunchResetInteractionState,
}
