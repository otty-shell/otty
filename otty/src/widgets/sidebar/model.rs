use iced::Point;

use super::types::SidebarItem;

/// Read-only view model for the sidebar presentation layer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarViewModel {
    pub(crate) active_item: SidebarItem,
    pub(crate) is_hidden: bool,
    pub(crate) is_workspace_open: bool,
    pub(crate) has_add_menu_open: bool,
    pub(crate) add_menu_cursor: Option<Point>,
}
