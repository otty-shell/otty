use iced::Point;

use crate::ui::widgets::{sidebar_menu, sidebar_workspace};

/// Events emitted by sidebar widgets and app-level sidebar interactions.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    Menu(sidebar_menu::SidebarMenuEvent),
    Workspace(sidebar_workspace::SidebarWorkspaceEvent),
    ToggleVisibility,
    PaneGridCursorMoved { position: Point },
    DismissAddMenu,
}
