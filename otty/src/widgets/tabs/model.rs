/// Tab entry projected for the tab bar view.
#[derive(Debug, Clone)]
pub(crate) struct TabBarItem {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) is_active: bool,
    pub(crate) is_hovered: bool,
    pub(crate) close_visible: bool,
}

/// View model for the tabs widget.
#[derive(Debug, Clone)]
pub(crate) struct TabsViewModel {
    pub(crate) tabs: Vec<TabBarItem>,
    pub(crate) active_tab_id: Option<u64>,
    pub(crate) hovered_tab_id: Option<u64>,
    pub(crate) has_tabs: bool,
}
