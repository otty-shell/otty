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
