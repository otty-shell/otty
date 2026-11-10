use otty_app_domain::mode::Mode;

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
    /// The pinned modes were written to `modes.json`, or the write failed
    /// with this error.
    PinnedSaved(Result<(), String>),
}

/// What the modes reducer reports: a switch, the pinned list to save, the
/// menu opening or closing, and a save that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModesOutcome {
    /// The active mode changed to this one; the frame swaps the left
    /// panel, main area, right panel and status bar.
    ModeChanged(Mode),
    /// The pinned list changed; save it to `modes.json` in this order.
    SavePinned(Vec<Mode>),
    /// The All modes menu opened. It takes the keyboard from the terminal,
    /// so typed keys do not reach the shell and Esc reaches the frame.
    MenuOpened,
    /// The All modes menu closed while this mode is active. Back in Terminal
    /// mode the active pane gets its keys back.
    MenuClosed(Mode),
    /// Saving the pinned modes failed: tell the user with this text. The
    /// switcher keeps what the user chose.
    PinnedSaveFailed(String),
}
