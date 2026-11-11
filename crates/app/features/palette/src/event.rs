use otty_app_domain::commands::AppCommand;
use otty_ui_kit::components::command_palette::PaletteAction;

/// What the command palette is asked to do.
#[derive(Debug, Clone)]
pub enum PaletteIntent {
    /// Open the palette (title bar search or the shortcut). While it is
    /// already open, only focus its input again.
    Open,
    /// A click outside the palette, Esc or another menu closes it.
    Dismiss,
    /// A click on the palette outside its input and entries; the input
    /// gets focus back so the keys keep working.
    FocusInput,
    /// Something the user did inside the palette.
    Action(PaletteAction),
}

/// What the command palette reports back to the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteOutcome {
    /// Give the palette input the keyboard focus: the palette opened, or
    /// asks for it again. Moving the focus there also takes it from the
    /// terminal panes.
    FocusInput,
    /// An open tab was picked: switch to it. The palette is already closed.
    SessionPicked {
        /// Id of the picked tab.
        tab_id: u64,
    },
    /// A launch was picked: run it, as the New session menu does. The
    /// palette is already closed.
    LaunchPicked {
        /// Path in the launch tree: the folder titles, then the launch
        /// title.
        path: Vec<String>,
    },
    /// An app command was picked: run it, as its shortcut would. It could
    /// run when it was picked; the palette is already closed.
    CommandPicked(AppCommand),
    /// The palette gave up the keyboard focus and nothing it ran takes it:
    /// in Terminal mode the active pane gets the keyboard back.
    KeyboardReleased,
}
