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
