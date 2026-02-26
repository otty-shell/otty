use iced::Point;

use crate::ui::widgets::{sidebar_menu, sidebar_workspace};

/// Reducer commands for sidebar widget.
#[derive(Debug, Clone)]
pub(crate) enum SidebarCommand {
    Menu(sidebar_menu::SidebarMenuEvent),
    Workspace(sidebar_workspace::SidebarWorkspaceEvent),
    ToggleVisibility,
    PaneGridCursorMoved { position: Point },
    DismissAddMenu,
}
