use std::fmt;
use std::path::PathBuf;

use iced::Point;
use iced::widget::{Id, pane_grid};
use otty_ui_term::BlockCommand;
use otty_ui_term::settings::Settings;

use super::types::{PaneMachine, TerminalEvent, TerminalKind};

/// What the terminal is asked to do: by its panes and the pane menu, by the
/// terminal widgets, by a shortcut, or by the binary on behalf of the tabs,
/// the frame and the settings.
#[derive(Clone)]
pub enum TerminalIntent {
    /// A terminal or command tab opened and is the active tab: start its
    /// terminal.
    OpenTab {
        /// Id of the tab.
        tab_id: u64,
        /// Title of the tab: the shell or launch name.
        title: String,
        /// Settings of the terminal, with the session it starts.
        settings: Box<Settings>,
        /// A shell or a command, and what a command does when it ends.
        kind: TerminalKind,
        /// The machine the pane headers name.
        machine: PaneMachine,
    },
    /// A tab came to the front, a terminal tab or not (the launch editor).
    TabActivated {
        /// Id of the tab.
        tab_id: u64,
    },
    /// A tab closed: its terminals end. `new_active_id` is the tab in front
    /// now; `None` when no tab is left.
    TabClosed {
        /// Id of the closed tab.
        tab_id: u64,
        /// Id of the tab that is active now.
        new_active_id: Option<u64>,
    },
    /// Terminal widget event forwarded from `otty_ui_term`.
    Widget(TerminalEvent),
    /// A pane was clicked.
    PaneClicked {
        /// Id of the tab.
        tab_id: u64,
        /// The pane.
        pane: pane_grid::Pane,
    },
    /// A pane resize was performed.
    PaneResized {
        /// Id of the tab.
        tab_id: u64,
        /// The split and its new ratio.
        event: pane_grid::ResizeEvent,
    },
    /// The cursor moved within the pane grid area.
    PaneGridCursorMoved {
        /// Id of the tab.
        tab_id: u64,
        /// Where the pointer is, in the pane grid.
        position: Point,
    },
    /// Open the context menu of a terminal pane.
    OpenContextMenu {
        /// Id of the tab.
        tab_id: u64,
        /// The pane.
        pane: pane_grid::Pane,
        /// The terminal in the pane.
        terminal_id: u64,
    },
    /// Close the context menu of a tab.
    CloseContextMenu {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Split the given pane along an axis.
    SplitPane {
        /// Id of the tab.
        tab_id: u64,
        /// The pane to split.
        pane: pane_grid::Pane,
        /// `Vertical` puts the new pane on the right, `Horizontal` below.
        axis: pane_grid::Axis,
    },
    /// Split the focused pane of the active tab along an axis. Nothing
    /// happens when the active tab is not a terminal tab.
    SplitFocusedPane {
        /// `Vertical` puts the new pane on the right, `Horizontal` below.
        axis: pane_grid::Axis,
    },
    /// Close the given pane.
    ClosePane {
        /// Id of the tab.
        tab_id: u64,
        /// The pane.
        pane: pane_grid::Pane,
    },
    /// Copy the current text selection from a terminal.
    CopySelection {
        /// Id of the tab.
        tab_id: u64,
        /// The terminal.
        terminal_id: u64,
    },
    /// Paste clipboard contents into the terminal prompt.
    PasteIntoPrompt {
        /// Id of the tab.
        tab_id: u64,
        /// The terminal.
        terminal_id: u64,
    },
    /// Copy the content of the selected block.
    CopySelectedBlockContent {
        /// Id of the tab.
        tab_id: u64,
        /// The terminal.
        terminal_id: u64,
    },
    /// Copy the prompt text of the selected block.
    CopySelectedBlockPrompt {
        /// Id of the tab.
        tab_id: u64,
        /// The terminal.
        terminal_id: u64,
    },
    /// Copy the command text of the selected block.
    CopySelectedBlockCommand {
        /// Id of the tab.
        tab_id: u64,
        /// The terminal.
        terminal_id: u64,
    },
    /// Give every open terminal a new color palette: the theme changed.
    ApplyTheme {
        /// The colors of the theme.
        palette: Box<otty_ui_term::ColorPalette>,
    },
    /// Close the context menus of every tab.
    CloseAllContextMenus,
    /// Give the focused pane of the active tab the keyboard: an overlay
    /// closed. An overlay still open keeps it, and outside Terminal mode no
    /// pane is on screen to take it.
    FocusActive,
    /// The mode on screen changed. Back in Terminal mode the panes take the
    /// current grid size, since the window may have changed meanwhile, and
    /// the active pane takes the keyboard.
    ModeChanged,
    /// The pane grid may have a new size (the window or a side panel
    /// changed): every tab takes it from the context.
    SyncPaneGridSize,
}

impl fmt::Debug for TerminalIntent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenTab {
                tab_id,
                title,
                kind,
                machine,
                ..
            } => f
                .debug_struct("OpenTab")
                .field("tab_id", tab_id)
                .field("title", title)
                .field("kind", kind)
                .field("machine", machine)
                .finish(),
            Self::TabActivated { tab_id } => f
                .debug_struct("TabActivated")
                .field("tab_id", tab_id)
                .finish(),
            Self::TabClosed {
                tab_id,
                new_active_id,
            } => f
                .debug_struct("TabClosed")
                .field("tab_id", tab_id)
                .field("new_active_id", new_active_id)
                .finish(),
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
            Self::SplitPane { tab_id, pane, axis } => f
                .debug_struct("SplitPane")
                .field("tab_id", tab_id)
                .field("pane", pane)
                .field("axis", axis)
                .finish(),
            Self::SplitFocusedPane { axis } => f
                .debug_struct("SplitFocusedPane")
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
            Self::ModeChanged => f.write_str("ModeChanged"),
            Self::SyncPaneGridSize => f.write_str("SyncPaneGridSize"),
        }
    }
}

/// What the terminal reports back to the frame: news for the tabs, the
/// modes and the explorer, a failure to tell, and keyboard and terminal
/// operations for the window.
#[derive(Debug, Clone)]
pub enum TerminalOutcome {
    /// The terminal of a new tab started, so the tab is really there and
    /// may bring Terminal mode to the front. A tab whose terminal can not
    /// start reports no start.
    TabStarted,
    /// The tab title (program of the focused pane) or its folder name
    /// changed: the tabs name the tab with it.
    TitleChanged {
        /// Id of the tab.
        tab_id: u64,
        /// The program of the focused pane.
        title: String,
        /// Folder name of the focused pane; `None` without one.
        folder: Option<String>,
    },
    /// Close the tab: its last pane closed, or its terminal could not
    /// start.
    CloseTab {
        /// Id of the tab.
        tab_id: u64,
    },
    /// The focused pane of the active tab runs on this computer in this
    /// folder: the Files section follows it.
    FolderFollowed {
        /// The folder the shell reported.
        cwd: PathBuf,
    },
    /// Give the keyboard to the terminal widget with this id.
    FocusTerminal(Id),
    /// Take the keyboard from every widget, the terminals too: a pane menu
    /// opened, so typed keys reach no shell under it and Esc reaches the
    /// frame.
    TakeKeyboard,
    /// Run a block command on the terminal widget with this id: select,
    /// clear the selection, copy or paste.
    RunBlockCommand {
        /// The widget id of the terminal.
        terminal: Id,
        /// What to run.
        command: BlockCommand,
    },
    /// Tell the user about a failure, such as a tab or a split whose
    /// terminal could not start; the text is in their language.
    Failed(String),
}
