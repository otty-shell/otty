use std::fmt;

use iced::Point;
use iced::widget::pane_grid;
use otty_ui_term::settings::Settings;

use super::model::TerminalKind;

/// Events emitted by terminal UI and terminal-related flows.
#[derive(Clone)]
pub(crate) enum TerminalEvent {
    OpenTab {
        tab_id: u64,
        terminal_id: u64,
        default_title: String,
        settings: Box<Settings>,
        kind: TerminalKind,
        sync_explorer: bool,
        error_tab: Option<(String, String)>,
    },
    TabClosed {
        tab_id: u64,
    },
    Widget(otty_ui_term::Event),
    PaneClicked {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    PaneResized {
        tab_id: u64,
        event: pane_grid::ResizeEvent,
    },
    PaneGridCursorMoved {
        tab_id: u64,
        position: Point,
    },
    OpenContextMenu {
        tab_id: u64,
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    CloseContextMenu {
        tab_id: u64,
    },
    ContextMenuInput {
        tab_id: u64,
    },
    SplitPane {
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    ClosePane {
        tab_id: u64,
        pane: pane_grid::Pane,
    },
    CopySelection {
        tab_id: u64,
        terminal_id: u64,
    },
    PasteIntoPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockContent {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockPrompt {
        tab_id: u64,
        terminal_id: u64,
    },
    CopySelectedBlockCommand {
        tab_id: u64,
        terminal_id: u64,
    },
    ApplyTheme {
        palette: Box<otty_ui_term::ColorPalette>,
    },
    CloseAllContextMenus,
    FocusActive,
    SyncSelection {
        tab_id: u64,
    },
}

impl fmt::Debug for TerminalEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerminalEvent::OpenTab {
                tab_id,
                terminal_id,
                default_title,
                kind,
                sync_explorer,
                ..
            } => f
                .debug_struct("OpenTab")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .field("default_title", default_title)
                .field("kind", kind)
                .field("sync_explorer", sync_explorer)
                .finish(),
            TerminalEvent::TabClosed { tab_id } => {
                f.debug_struct("TabClosed").field("tab_id", tab_id).finish()
            },
            TerminalEvent::Widget(event) => {
                f.debug_tuple("Widget").field(event).finish()
            },
            TerminalEvent::PaneClicked { tab_id, pane } => f
                .debug_struct("PaneClicked")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            TerminalEvent::PaneResized { tab_id, event } => f
                .debug_struct("PaneResized")
                .field("tab_id", tab_id)
                .field("event", event)
                .finish(),
            TerminalEvent::PaneGridCursorMoved { tab_id, position } => f
                .debug_struct("PaneGridCursorMoved")
                .field("tab_id", tab_id)
                .field("position", position)
                .finish(),
            TerminalEvent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => f
                .debug_struct("OpenContextMenu")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CloseContextMenu { tab_id } => f
                .debug_struct("CloseContextMenu")
                .field("tab_id", tab_id)
                .finish(),
            TerminalEvent::ContextMenuInput { tab_id } => f
                .debug_struct("ContextMenuInput")
                .field("tab_id", tab_id)
                .finish(),
            TerminalEvent::SplitPane { tab_id, pane, axis } => f
                .debug_struct("SplitPane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("axis", axis)
                .finish(),
            TerminalEvent::ClosePane { tab_id, pane } => f
                .debug_struct("ClosePane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            TerminalEvent::CopySelection {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelection")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("PasteIntoPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockContent")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockCommand")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            TerminalEvent::ApplyTheme { .. } => f.write_str("ApplyTheme"),
            TerminalEvent::CloseAllContextMenus => {
                f.write_str("CloseAllContextMenus")
            },
            TerminalEvent::FocusActive => f.write_str("FocusActive"),
            TerminalEvent::SyncSelection { tab_id } => f
                .debug_struct("SyncSelection")
                .field("tab_id", tab_id)
                .finish(),
        }
    }
}
