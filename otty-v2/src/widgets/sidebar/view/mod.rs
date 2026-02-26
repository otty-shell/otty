use iced::{Element, Size, Theme};

use super::event::SidebarUiEvent;
pub(crate) use crate::ui::widgets::sidebar_menu::SidebarMenuProps;
pub(crate) use crate::ui::widgets::sidebar_workspace::SidebarWorkspaceProps;
pub(crate) use crate::ui::widgets::sidebar_workspace_add_menu::SidebarWorkspaceAddMenuProps;
use crate::ui::widgets::{
    sidebar_menu, sidebar_workspace, sidebar_workspace_add_menu,
};

/// Render sidebar menu rail and map to sidebar UI events.
pub(crate) fn menu_rail<'a>(
    props: SidebarMenuProps<'a>,
) -> Element<'a, SidebarUiEvent> {
    sidebar_menu::view(props).map(SidebarUiEvent::Menu)
}

/// Render sidebar workspace host and map to sidebar UI events.
pub(crate) fn workspace_host<'a>(
    props: SidebarWorkspaceProps<'a>,
) -> Element<'a, SidebarUiEvent, Theme, iced::Renderer> {
    sidebar_workspace::view(props).map(SidebarUiEvent::Workspace)
}

/// Render sidebar add-menu overlay and map to sidebar UI events.
pub(crate) fn add_menu_overlay<'a>(
    props: SidebarWorkspaceAddMenuProps<'a>,
) -> Element<'a, SidebarUiEvent> {
    sidebar_workspace_add_menu::view(props).map(SidebarUiEvent::Workspace)
}

/// Build a UI event for sidebar split resize.
pub(crate) fn resize_event(
    event: iced::widget::pane_grid::ResizeEvent,
) -> SidebarUiEvent {
    SidebarUiEvent::Menu(sidebar_menu::SidebarMenuEvent::Resized(event))
}

/// Build a UI event for cursor move in the sidebar workspace area.
pub(crate) fn cursor_moved(position: iced::Point) -> SidebarUiEvent {
    SidebarUiEvent::PaneGridCursorMoved { position }
}

/// Build sidebar add-menu props from explicit values.
pub(crate) fn add_menu_props<'a>(
    cursor: iced::Point,
    theme: crate::theme::ThemeProps<'a>,
    area_size: Size,
) -> SidebarWorkspaceAddMenuProps<'a> {
    SidebarWorkspaceAddMenuProps {
        cursor,
        theme,
        area_size,
    }
}
