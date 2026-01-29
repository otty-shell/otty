use iced::{Element, Theme};

use crate::features::quick_commands::event::QuickCommandsEvent;
use crate::state::{SidebarItem, State};
use crate::theme::ThemeProps;

mod terminal;

/// Events emitted by sidebar workspace content.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    TerminalNewTab,
    QuickCommands(QuickCommandsEvent),
}

/// Render the workspace content based on the active sidebar item.
pub(crate) fn view<'a>(
    state: &'a State,
    theme: ThemeProps<'a>,
) -> Element<'a, Event, Theme, iced::Renderer> {
    match state.sidebar.active_item {
        SidebarItem::Terminal => terminal::view(terminal::Props {
            theme,
            quick_commands: &state.quick_commands,
            workspace_size: state.sidebar_workspace_size(),
        }),
    }
}
