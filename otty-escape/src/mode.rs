use bitflags::bitflags;

/// Wrapper for the ANSI modes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Mode {
    /// Known ANSI mode.
    Named(NamedMode),
    /// Unidentified publc mode.
    Unknown(u16),
}

impl Mode {
    pub(crate) fn from_raw(mode: u16) -> Self {
        match mode {
            4 => Self::Named(NamedMode::Insert),
            20 => Self::Named(NamedMode::LineFeedNewLine),
            _ => Self::Unknown(mode),
        }
    }

    /// Get the raw value of the mode.
    pub fn raw(self) -> u16 {
        match self {
            Self::Named(named) => named as u16,
            Self::Unknown(mode) => mode,
        }
    }
}

impl From<NamedMode> for Mode {
    fn from(value: NamedMode) -> Self {
        Self::Named(value)
    }
}

/// ANSI modes.
#[repr(u16)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NamedMode {
    /// IRM Insert Mode.
    Insert = 4,
    LineFeedNewLine = 20,
}

/// Wrapper for the private DEC modes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PrivateMode {
    /// Known private mode.
    Named(NamedPrivateMode),
    /// Unknown private mode.
    Unknown(u16),
}

impl PrivateMode {
    pub(crate) fn from_raw(mode: u16) -> Self {
        match mode {
            1 => Self::Named(NamedPrivateMode::CursorKeys),
            3 => Self::Named(NamedPrivateMode::ColumnMode),
            6 => Self::Named(NamedPrivateMode::Origin),
            7 => Self::Named(NamedPrivateMode::LineWrap),
            12 => Self::Named(NamedPrivateMode::BlinkingCursor),
            25 => Self::Named(NamedPrivateMode::ShowCursor),
            1000 => Self::Named(NamedPrivateMode::ReportMouseClicks),
            1002 => Self::Named(NamedPrivateMode::ReportCellMouseMotion),
            1003 => Self::Named(NamedPrivateMode::ReportAllMouseMotion),
            1004 => Self::Named(NamedPrivateMode::ReportFocusInOut),
            1005 => Self::Named(NamedPrivateMode::Utf8Mouse),
            1006 => Self::Named(NamedPrivateMode::SgrMouse),
            1007 => Self::Named(NamedPrivateMode::AlternateScroll),
            1042 => Self::Named(NamedPrivateMode::UrgencyHints),
            1049 => {
                Self::Named(NamedPrivateMode::SwapScreenAndSetRestoreCursor)
            },
            2004 => Self::Named(NamedPrivateMode::BracketedPaste),
            2026 => Self::Named(NamedPrivateMode::SyncUpdate),
            _ => Self::Unknown(mode),
        }
    }

    /// Get the raw value of the mode.
    pub fn raw(self) -> u16 {
        match self {
            Self::Named(named) => named as u16,
            Self::Unknown(mode) => mode,
        }
    }
}

impl From<NamedPrivateMode> for PrivateMode {
    fn from(value: NamedPrivateMode) -> Self {
        Self::Named(value)
    }
}

/// Private DEC modes.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NamedPrivateMode {
    CursorKeys = 1,
    /// Select 80 or 132 columns per page (DECCOLM).
    ///
    /// CSI ? 3 h -> set 132 column font.
    /// CSI ? 3 l -> reset 80 column font.
    ///
    /// Additionally,
    ///
    /// * set margins to default positions
    /// * erases all data in page memory
    /// * resets DECLRMM to unavailable
    /// * clears data from the status line (if set to host-writable)
    ColumnMode = 3,
    Origin = 6,
    LineWrap = 7,
    BlinkingCursor = 12,
    ShowCursor = 25,
    ReportMouseClicks = 1000,
    ReportCellMouseMotion = 1002,
    ReportAllMouseMotion = 1003,
    ReportFocusInOut = 1004,
    Utf8Mouse = 1005,
    SgrMouse = 1006,
    AlternateScroll = 1007,
    UrgencyHints = 1042,
    SwapScreenAndSetRestoreCursor = 1049,
    BracketedPaste = 2004,
    /// The mode is handled automatically by [`Processor`].
    SyncUpdate = 2026,
}

/// Mode for clearing line.
///
/// Relative to cursor.
#[derive(Debug)]
pub enum LineClearMode {
    /// Clear right of cursor.
    Right,
    /// Clear left of cursor.
    Left,
    /// Clear entire line.
    All,
}

/// Mode for clearing terminal.
///
/// Relative to cursor.
#[derive(Debug)]
pub enum ClearMode {
    /// Clear below cursor.
    Below,
    /// Clear above cursor.
    Above,
    /// Clear entire terminal.
    All,
    /// Clear 'saved' lines (scrollback).
    Saved,
}

/// Mode for clearing tab stops.
#[derive(Debug)]
pub enum TabClearMode {
    /// Clear stop under cursor.
    Current,
    /// Clear all stops.
    All,
}

/// SCP control's first parameter which determines character path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScpCharPath {
    /// SCP's first parameter value of 0. Behavior is implementation defined.
    Default,
    /// SCP's first parameter value of 1 which sets character path to
    /// LEFT-TO-RIGHT.
    LTR,
    /// SCP's first parameter value of 2 which sets character path to
    /// RIGHT-TO-LEFT.
    RTL,
}

/// SCP control's second parameter which determines update mode/direction
/// between components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScpUpdateMode {
    /// SCP's second parameter value of 0 (the default). Implementation
    /// dependant update.
    ImplementationDependant,
    /// SCP's second parameter value of 1.
    ///
    /// Reflect data component changes in the presentation component.
    DataToPresentation,
    /// SCP's second parameter value of 2.
    ///
    /// Reflect presentation component changes in the data component.
    PresentationToData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifyOtherKeys {
    /// Reset the state.
    Reset,
    /// Enables this feature except for keys with well-known behavior, e.g.,
    /// Tab, Backspace and some special control character cases which are
    /// built into the X11 library (e.g., Control-Space to make a NUL, or
    /// Control-3 to make an Escape character).
    ///
    /// Escape sequences shouldn't be emitted under the following circumstances:
    /// - When the key is in range of `[64;127]` and the modifier is either
    ///   Control or Shift
    /// - When the key combination is a known control combination alias
    ///
    /// For more details, consult the [`example`] for the suggested translation.
    ///
    /// [`example`]: https://github.com/alacritty/vte/blob/master/doc/modifyOtherKeys-example.txt
    EnableExceptWellDefined,
    /// Enables this feature for all keys including the exceptions of
    /// [`Self::EnableExceptWellDefined`].  XTerm still ignores the special
    /// cases built into the X11 library. Any shifted (modified) ordinary
    /// key send an escape sequence. The Alt- and Meta- modifiers cause
    /// XTerm to send escape sequences.
    ///
    /// For more details, consult the [`example`] for the suggested translation.
    ///
    /// [`example`]: https://github.com/alacritty/vte/blob/master/doc/modifyOtherKeys-example.txt
    EnableAll,
}

bitflags! {
    /// A set of [`kitty keyboard protocol'] modes.
    ///
    /// [`kitty keyboard protocol']: https://sw.kovidgoyal.net/kitty/keyboard-protocol
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct KeyboardModes : u8 {
        /// No keyboard protocol mode is set.
        const NO_MODE                 = 0b0000_0000;
        /// Report `Esc`, `alt` + `key`, `ctrl` + `key`, `ctrl` + `alt` + `key`, `shift`
        /// + `alt` + `key` keys using `CSI u` sequence instead of raw ones.
        const DISAMBIGUATE_ESC_CODES  = 0b0000_0001;
        /// Report key presses, release, and repetition alongside the escape. Key events
        /// that result in text are reported as plain UTF-8, unless the
        /// [`Self::REPORT_ALL_KEYS_AS_ESC`] is enabled.
        const REPORT_EVENT_TYPES      = 0b0000_0010;
        /// Additionally report shifted key an dbase layout key.
        const REPORT_ALTERNATE_KEYS   = 0b0000_0100;
        /// Report every key as an escape sequence.
        const REPORT_ALL_KEYS_AS_ESC  = 0b0000_1000;
        /// Report the text generated by the key event.
        const REPORT_ASSOCIATED_TEXT  = 0b0001_0000;
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardModesApplyBehavior {
    /// Replace the active flags with the new ones.
    #[default]
    Replace,
    /// Merge the given flags with currently active ones.
    Union,
    /// Remove the given flags from the active ones.
    Difference,
}
