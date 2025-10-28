use cursor_icon::CursorIcon;

use crate::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, CursorShape,
    CursorStyle, Hyperlink, LineClearMode, Mode, PrivateMode, Rgb,
    TabClearMode,
    keyboard::{KeyboardMode, KeyboardModeApplyBehavior, ModifyOtherKeysState},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Print(char),
    Bell,
    SetHyperlink(Option<Hyperlink>),
    SetCharacterAttribute(CharacterAttribute),
    /// Domain specific actions
    Control(TerminalControlAction),
    Edit(EditAction),
    Cursor(CursorAction),
}

impl Action {
    pub fn is_control(&self) -> bool {
        match self {
            Self::Control(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CursorAction {
    /// Set the text cursor shape
    SetCursorShape(CursorShape),
    /// Set the cursor pointer type [https://www.w3.org/TR/css-ui-3/#cursor]
    SetCursorIcon(CursorIcon),
    SetCursorStyle(Option<CursorStyle>),
    SaveCursorPosition,
    RestoreCursorPosition,
    MoveUp {
        rows: usize,
        carrage_return_needed: bool,
    },
    MoveDown {
        rows: usize,
        carrage_return_needed: bool,
    },
    MoveForward(usize),
    MoveBackward(usize),
    Goto(i32, usize),
    GotoRow(i32),
    GotoColumn(usize),
}

impl Into<Action> for CursorAction {
    fn into(self) -> Action {
        Action::Cursor(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EditAction {
    InsertBlank(usize),
    InsertBlankLines(usize),
    DeleteLines(usize),
    DeleteChars(usize),
    EraseChars(usize),
    Backspace,
    CarriageReturn,
    LineFeed,
    NextLine,
    NewLine,
    Substitute,
    SetHorizontalTab,
    ReverseIndex,
    ResetState,
    ClearScreen(ClearMode),
    ClearLine(LineClearMode),
    InsertTabs(u16),
    SetTabs(u16),
    ClearTabs(TabClearMode),
    ScreenAlignmentDisplay,
    MoveForwardTabs(u16),
    MoveBackwardTabs(u16),
    /// Charset specific actions
    ///
    SetActiveCharsetIndex(CharsetIndex),
    ConfigureCharset(Charset, CharsetIndex),
    /// Color specific actions
    ///
    SetColor {
        index: usize,
        color: Rgb,
    },
    QueryColor(usize),
    ResetColor(usize),
    /// Scrolling specific actions
    ///
    SetScrollingRegion(usize, usize),
    ScrollUp(usize),
    ScrollDown(usize),
}

impl Into<Action> for EditAction {
    fn into(self) -> Action {
        Action::Edit(self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TerminalControlAction {
    IdentifyTerminal(Option<char>),
    ReportDeviceStatus(usize),
    /// Keyboard specific actions
    ///
    SetKeypadApplicationMode,
    UnsetKeypadApplicationMode,
    // StoreToClipboard(ClipboardType, Vec<u8>),
    SetModifyOtherKeysState(ModifyOtherKeysState),
    ReportModifyOtherKeysState,
    ReportKeyboardMode,
    SetKeyboardMode(KeyboardMode, KeyboardModeApplyBehavior),
    PushKeyboardMode(KeyboardMode),
    PopKeyboardModes(u16),
    /// Terminal mode specific actions
    ///
    SetMode(Mode),
    SetPrivateMode(PrivateMode),
    UnsetMode(Mode),
    UnsetPrivateMode(PrivateMode),
    ReportMode(Mode),
    ReportPrivateMode(PrivateMode),
    /// Window manipulation specific actions
    ///
    RequestTextAreaSizeByPixels,
    RequestTextAreaSizeByChars,
    PushWindowTitle,
    PopWindowTitle,
    SetWindowTitle(String),
}

impl Into<Action> for TerminalControlAction {
    fn into(self) -> Action {
        Action::Control(self)
    }
}

pub trait Actor {
    /// processing the action
    fn handle(&mut self, _: Action) {}
    /// Begin synchronized (batch) update (DEC mode 2026)
    fn begin_sync(&mut self) {}
    /// End synchronized (batch) update (DEC mode 2026)
    fn end_sync(&mut self) {}
}
