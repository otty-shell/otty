use std::fmt;

use iced::Point;
use iced::widget::pane_grid;
use otty_ui_term::settings::Settings;

use super::model::TerminalKind;

/// Intent events handled by the terminal workspace presentation layer.
#[derive(Clone)]
pub(crate) enum TerminalWorkspaceIntent {
    /// Request to open a new terminal tab.
    OpenTab {
        tab_id: u64,
        default_title: String,
        settings: Box<Settings>,
        kind: TerminalKind,
        sync_explorer: bool,
    },
    /// Notification that a tab has been closed externally.
    TabClosed { tab_id: u64 },
    /// Terminal widget event forwarded from `otty_ui_term`.
    Widget(otty_ui_term::Event),
    /// A pane was clicked.
    PaneClicked { tab_id: u64, pane: pane_grid::Pane },
    /// A pane resize was performed.
    PaneResized {
        tab_id: u64,
        event: pane_grid::ResizeEvent,
    },
    /// The cursor moved within the pane grid area.
    PaneGridCursorMoved { tab_id: u64, position: Point },
    /// Request to open a context menu for a terminal pane.
    OpenContextMenu {
        tab_id: u64,
        pane: pane_grid::Pane,
        terminal_id: u64,
    },
    /// Dismiss the context menu for a tab.
    CloseContextMenu { tab_id: u64 },
    /// Keyboard input while context menu is focused (absorb focus trap).
    ContextMenuInput { tab_id: u64 },
    /// Split the given pane along an axis.
    SplitPane {
        tab_id: u64,
        pane: pane_grid::Pane,
        axis: pane_grid::Axis,
    },
    /// Close the given pane.
    ClosePane { tab_id: u64, pane: pane_grid::Pane },
    /// Copy the current text selection from a terminal.
    CopySelection { tab_id: u64, terminal_id: u64 },
    /// Paste clipboard contents into the terminal prompt.
    PasteIntoPrompt { tab_id: u64, terminal_id: u64 },
    /// Copy the content of the selected block.
    CopySelectedBlockContent { tab_id: u64, terminal_id: u64 },
    /// Copy the prompt text of the selected block.
    CopySelectedBlockPrompt { tab_id: u64, terminal_id: u64 },
    /// Copy the command text of the selected block.
    CopySelectedBlockCommand { tab_id: u64, terminal_id: u64 },
    /// Apply a new terminal color palette across all tabs.
    ApplyTheme {
        palette: Box<otty_ui_term::ColorPalette>,
    },
    /// Close all context menus across all tabs.
    CloseAllContextMenus,
    /// Request focus on the active terminal pane.
    FocusActive,
    /// Synchronise block selection state for a tab.
    SyncSelection { tab_id: u64 },
    /// Synchronise pane grid size across all tabs from current layout context.
    SyncPaneGridSize,
}

impl fmt::Debug for TerminalWorkspaceIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenTab {
                tab_id,
                default_title,
                kind,
                sync_explorer,
                ..
            } => f
                .debug_struct("OpenTab")
                .field("tab_id", tab_id)
                .field("default_title", default_title)
                .field("kind", kind)
                .field("sync_explorer", sync_explorer)
                .finish(),
            Self::TabClosed { tab_id } => {
                f.debug_struct("TabClosed").field("tab_id", tab_id).finish()
            },
            Self::Widget(event) => {
                f.debug_tuple("Widget").field(event).finish()
            },
            Self::PaneClicked { tab_id, pane } => f
                .debug_struct("PaneClicked")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            Self::PaneResized { tab_id, event } => f
                .debug_struct("PaneResized")
                .field("tab_id", tab_id)
                .field("event", event)
                .finish(),
            Self::PaneGridCursorMoved { tab_id, position } => f
                .debug_struct("PaneGridCursorMoved")
                .field("tab_id", tab_id)
                .field("position", position)
                .finish(),
            Self::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            } => f
                .debug_struct("OpenContextMenu")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::CloseContextMenu { tab_id } => f
                .debug_struct("CloseContextMenu")
                .field("tab_id", tab_id)
                .finish(),
            Self::ContextMenuInput { tab_id } => f
                .debug_struct("ContextMenuInput")
                .field("tab_id", tab_id)
                .finish(),
            Self::SplitPane { tab_id, pane, axis } => f
                .debug_struct("SplitPane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("axis", axis)
                .finish(),
            Self::ClosePane { tab_id, pane } => f
                .debug_struct("ClosePane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .finish(),
            Self::CopySelection {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelection")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::PasteIntoPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("PasteIntoPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::CopySelectedBlockContent {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockContent")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::CopySelectedBlockPrompt {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockPrompt")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::CopySelectedBlockCommand {
                tab_id,
                terminal_id,
            } => f
                .debug_struct("CopySelectedBlockCommand")
                .field("tab_id", tab_id)
                .field("terminal_id", terminal_id)
                .finish(),
            Self::ApplyTheme { .. } => f.write_str("ApplyTheme"),
            Self::CloseAllContextMenus => f.write_str("CloseAllContextMenus"),
            Self::FocusActive => f.write_str("FocusActive"),
            Self::SyncSelection { tab_id } => f
                .debug_struct("SyncSelection")
                .field("tab_id", tab_id)
                .finish(),
            Self::SyncPaneGridSize => f.write_str("SyncPaneGridSize"),
        }
    }
}

/// Effect events produced by the terminal workspace reducer.
#[derive(Debug, Clone)]
pub(crate) enum TerminalWorkspaceEffect {
    /// A terminal tab was closed (last pane shut down).
    TabClosed { tab_id: u64 },
    /// The tab title changed.
    TitleChanged { tab_id: u64, title: String },
    /// Request the explorer to sync from the active terminal CWD.
    SyncExplorer,
}

/// Terminal workspace event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum TerminalWorkspaceEvent {
    /// Intent event reduced by the terminal workspace widget.
    Intent(TerminalWorkspaceIntent),
    /// External effect orchestrated by app-level routing.
    Effect(TerminalWorkspaceEffect),
}
