/// Commands accepted by the sidebar reducer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarCommand {
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
