use super::types::HeaderMenu;

/// Intent events handled by the sidebar presentation layer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarIntent {
    /// Show or hide the left panel (leaves focus mode first). It shows at the
    /// width it had.
    ToggleLeft,
    /// Show or hide the right panel (leaves focus mode first). It shows at
    /// the width it had.
    ToggleRight,
    /// Enter or leave focus mode.
    ToggleFocusMode,
    /// Open or close the Sessions section of the left panel.
    ToggleSessions,
    /// Open or close the Launches section of the left panel.
    ToggleLaunches,
    /// Make the Launches section visible: leave focus mode, show the left
    /// panel at the width it had and open the section. Parts already shown
    /// stay as they are.
    RevealLaunches,
    /// Make the right panel visible at the width it had: leave focus mode and
    /// show the panel. A panel already shown stays as it is.
    RevealRight,
    /// Open a header menu at the cursor, closing the other one.
    HeaderMenuOpen(HeaderMenu),
    /// Close the open header menu.
    HeaderMenuDismiss,
    WorkspaceCursorMoved {
        position: iced::Point,
    },
    /// A panel edge was dragged; the split tells which side.
    Resized(iced::widget::pane_grid::ResizeEvent),
    /// A new frame was drawn while the focus mode note is shown.
    FocusNoteFrame(iced::time::Instant),
}

/// Effect events produced by the sidebar reducer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEffect {
    SyncTerminalGridSizes,
    QuickLaunchResetInteractionState,
    /// The open header menu closed. Back in Terminal mode the active pane
    /// gets its keys back.
    HeaderMenuClosed,
}

/// Sidebar event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    /// Intent event reduced by the sidebar widget.
    Intent(SidebarIntent),
    /// External effect orchestrated by app-level routing.
    Effect(SidebarEffect),
}
