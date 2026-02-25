use iced::{Element, Point, Theme};

use crate::features::explorer::{ExplorerEvent, ExplorerState};
use crate::features::quick_launches::{QuickLaunchEvent, QuickLaunchState};
use crate::theme::ThemeProps;
use crate::ui::widgets::{
    sidebar_workspace_explorer, sidebar_workspace_terminal,
};

/// Events emitted by sidebar workspace widget.
#[derive(Debug, Clone)]
pub(crate) enum SidebarWorkspaceEvent {
    TerminalAddMenuOpen,
    TerminalAddMenuDismiss,
    TerminalAddMenuAction(SidebarWorkspaceAddMenuAction),
    WorkspaceCursorMoved { position: Point },
    QuickLaunch(QuickLaunchEvent),
    Explorer(ExplorerEvent),
}

/// Actions emitted by sidebar workspace add menu.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SidebarWorkspaceAddMenuAction {
    CreateTab,
    CreateCommand,
    CreateFolder,
}

/// Active section rendered in the sidebar workspace area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarWorkspaceItem {
    Terminal,
    Explorer,
}

/// Props for rendering sidebar workspace.
#[derive(Clone, Copy)]
pub(crate) struct SidebarWorkspaceProps<'a> {
    pub(crate) active_item: SidebarWorkspaceItem,
    pub(crate) quick_launches: &'a QuickLaunchState,
    pub(crate) explorer: &'a ExplorerState,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the workspace content based on the active sidebar item.
pub(crate) fn view<'a>(
    props: SidebarWorkspaceProps<'a>,
) -> Element<'a, SidebarWorkspaceEvent, Theme, iced::Renderer> {
    match props.active_item {
        SidebarWorkspaceItem::Terminal => sidebar_workspace_terminal::view(
            sidebar_workspace_terminal::SidebarWorkspaceTerminalProps {
                theme: props.theme,
                quick_launches: props.quick_launches,
            },
        ),
        SidebarWorkspaceItem::Explorer => sidebar_workspace_explorer::view(
            sidebar_workspace_explorer::SidebarWorkspaceExplorerProps {
                theme: props.theme,
                explorer: props.explorer,
            },
        )
        .map(SidebarWorkspaceEvent::Explorer),
    }
}
