/// Intent events handled by the sidebar presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarIntent {
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

/// Sidebar event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    /// Intent event reduced by the sidebar widget.
    Intent(SidebarIntent),
    /// External effect orchestrated by app-level routing.
    Effect(SidebarEffect),
}
