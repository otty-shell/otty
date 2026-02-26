use iced::Point;
use iced::widget::pane_grid;
use otty_ui_term::settings::Settings;

use super::model::TerminalKind;

/// Internal commands dispatched to the terminal workspace reducer.
#[derive(Clone)]
pub(crate) enum TerminalWorkspaceCommand {
    /// Open a new terminal tab.
    OpenTab {
        tab_id: u64,
        terminal_id: u64,
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
}
