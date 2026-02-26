/// Fixed width of the sidebar menu rail.
pub(crate) const SIDEBAR_MENU_WIDTH: f32 = 52.0;

/// Sidebar menu destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarItem {
    Terminal,
    Explorer,
}

/// Pane slots in the sidebar split view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarPane {
    Workspace,
    Content,
}

/// Read-only view model for sidebar rendering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewModel {
    pub(crate) active_item: SidebarItem,
    pub(crate) is_hidden: bool,
    pub(crate) is_workspace_open: bool,
}
