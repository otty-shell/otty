use iced::{Element, Point, Theme};

use crate::features::explorer::event::ExplorerEvent;
use crate::features::quick_commands::event::QuickCommandsEvent;
use crate::state::{SidebarItem, State};
use crate::theme::ThemeProps;

pub(crate) mod add_menu;
mod explorer;
mod terminal;

/// Events emitted by sidebar workspace content.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    TerminalAddMenuOpen,
    TerminalAddMenuDismiss,
    TerminalAddMenuAction(AddMenuAction),
    WorkspaceCursorMoved { position: Point },
    QuickCommands(QuickCommandsEvent),
    Explorer(ExplorerEvent),
}

/// Actions emitted by the terminal add menu.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AddMenuAction {
    CreateTab,
    CreateCommand,
    CreateFolder,
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
        }),
        SidebarItem::Explorer => explorer::view(explorer::Props {
            theme,
            explorer: &state.explorer,
        })
        .map(Event::Explorer),
    }
}
