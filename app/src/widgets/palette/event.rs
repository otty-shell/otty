use otty_ui_kit::components::command_palette::PaletteAction;

use super::types::PaletteTarget;

/// Intent events handled by the command palette.
#[derive(Debug, Clone)]
pub(crate) enum PaletteIntent {
    /// Open the palette (title bar search or the shortcut). While it is
    /// already open, only focus its input again.
    Open,
    /// A click outside the palette closes it.
    Dismiss,
    /// A click on the palette outside its input and entries; the input
    /// gets focus back so the keys keep working.
    FocusInput,
    /// Something the user did inside the palette.
    Action(PaletteAction),
}

/// Effect events produced by the palette reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PaletteEffect {
    /// An entry was chosen; the palette is already closed.
    Run(PaletteTarget),
    /// The palette was closed without running anything.
    Dismissed,
}

/// Palette event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum PaletteEvent {
    /// Intent event reduced by the palette widget.
    Intent(PaletteIntent),
    /// External effect orchestrated by app-level routing.
    Effect(PaletteEffect),
}
