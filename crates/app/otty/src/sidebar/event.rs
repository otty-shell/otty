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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the containers feature (issue 13) sends it when a row opens the inspector"
        )
    )]
    RevealRight,
    /// Open a header menu at the cursor, closing the other one.
    HeaderMenuOpen(HeaderMenu),
    /// Close the open header menu.
    HeaderMenuDismiss,
    /// The pointer moved over the window content; header menus open here.
    WorkspaceCursorMoved { position: iced::Point },
    /// A panel edge was dragged; the split tells which side.
    Resized(iced::widget::pane_grid::ResizeEvent),
    /// A new frame was drawn while the focus mode note is shown.
    FocusNoteFrame(iced::time::Instant),
}

/// What the sidebar reducer reports: news for the features around the
/// panels and the keyboard focus a header menu takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SidebarOutcome {
    /// The main area changed size, so the terminals redraw at their new
    /// size.
    MainAreaResized,
    /// A panel edge is being dragged, so the hover effects of the Launches
    /// tree wait until the drag settles.
    PanelEdgeDragged,
    /// A header menu opened. It takes the keyboard from the terminal, so
    /// typed keys do not reach the shell and Esc reaches the frame.
    HeaderMenuOpened,
    /// The open header menu closed. Back in Terminal mode the active pane
    /// gets its keys back.
    HeaderMenuClosed,
}
