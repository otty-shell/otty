use iced::Point;

use crate::ui::widgets::{sidebar_menu, sidebar_workspace};
use crate::widgets::explorer::ExplorerUiEvent;
use crate::widgets::quick_launch::QuickLaunchEvent;

/// Events emitted by sidebar presentation and user interactions.
#[derive(Debug, Clone)]
pub(crate) enum SidebarUiEvent {
    Menu(sidebar_menu::SidebarMenuEvent),
    Workspace(sidebar_workspace::SidebarWorkspaceEvent),
    ToggleVisibility,
    PaneGridCursorMoved { position: Point },
    DismissAddMenu,
}

/// Side-effect events emitted by sidebar reducer.
#[derive(Debug, Clone)]
pub(crate) enum SidebarEffectEvent {
    SyncTerminalGridSizes,
    OpenSettingsTab,
    OpenTerminalTab,
    QuickLaunch(QuickLaunchEvent),
    Explorer(ExplorerUiEvent),
}
