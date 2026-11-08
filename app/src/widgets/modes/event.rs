use crate::domain::mode::Mode;

/// Intent events handled by the modes widget.
#[derive(Debug, Clone)]
pub(crate) enum ModesIntent {
    /// Switch to this mode (a switcher tab, a menu row or the command
    /// palette). Also closes the All modes menu.
    Select(Mode),
    /// Switch to the mode at this switcher position (a mode shortcut):
    /// Terminal at 0, then the pinned modes. Past the pinned modes nothing
    /// happens.
    SelectAt { position: usize },
    /// Pin this mode; refused for Terminal, an already pinned mode, or
    /// when every pin slot is taken.
    Pin(Mode),
    /// Unpin this mode. The active mode stays active, as a temporary tab.
    Unpin(Mode),
    /// Open or close the All modes menu (the "⋯" button or its shortcut).
    ToggleMenu,
    /// Close the All modes menu (Esc or a click outside it).
    CloseMenu,
}

/// Effect events produced by the modes reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModesEffect {
    /// The active mode changed to this one; the root view swaps the left
    /// panel, main area, right panel and status bar.
    ModeChanged(Mode),
    /// The pinned list changed; save it to `modes.json` in this order.
    SavePinned(Vec<Mode>),
}

/// Modes event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ModesEvent {
    /// Intent event reduced by the modes widget.
    Intent(ModesIntent),
    /// External effect orchestrated by app-level routing.
    Effect(ModesEffect),
}
