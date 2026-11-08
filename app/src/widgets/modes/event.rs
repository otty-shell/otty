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
    /// A tab came to the front: it opened or became active. Tabs show only
    /// in Terminal mode, so another mode switches to Terminal and closes the
    /// menu, as [`ModesIntent::Select`] does. In Terminal mode nothing
    /// changes, and an open menu stays open.
    TabShown,
}

/// Effect events produced by the modes reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModesEffect {
    /// The active mode changed to this one; the root view swaps the left
    /// panel, main area, right panel and status bar.
    ModeChanged(Mode),
    /// The pinned list changed; save it to `modes.json` in this order.
    SavePinned(Vec<Mode>),
    /// The All modes menu opened. It takes the keyboard from the terminal,
    /// so typed keys do not reach the shell and Esc reaches the app.
    MenuOpened,
    /// The All modes menu closed while this mode is active. Back in Terminal
    /// mode the active pane gets its keys back.
    MenuClosed(Mode),
}

/// Modes event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ModesEvent {
    /// Intent event reduced by the modes widget.
    Intent(ModesIntent),
    /// External effect orchestrated by app-level routing.
    Effect(ModesEffect),
}
