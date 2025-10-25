use cursor_icon::CursorIcon;

use crate::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, CursorShape,
    CursorStyle, Hyperlink, LineClearMode, Mode, PrivateMode, Rgb, ScpCharPath,
    ScpUpdateMode, TabClearMode,
    clipboard::ClipboardType,
    keyboard::{KeyboardMode, KeyboardModeApplyBehavior, ModifyOtherKeysState},
};

#[derive(Debug)]
pub enum Action {
    Print(char),
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
    Bell,
    Substitute,
    SetHorizontalTab,
    ReverseIndex,
    IdentifyTerminal(Option<char>),
    SetHyperlink(Option<Hyperlink>),
    ResetState,
    ScreenAlignmentDisplay,
    ClearScreen(ClearMode),
    ClearLine(LineClearMode),
    InsertTabs(u16),
    SetTabs(u16),
    ClearTabs(TabClearMode),
    MoveForwardTabs(u16),
    MoveBackwardTabs(u16),
    SetCharacterAttribute(CharacterAttribute),
    SetSCP(ScpCharPath, ScpUpdateMode),
    ReportDeviceStatus(usize),
    /// Charset specific actions
    ///
    SetActiveCharsetIndex(CharsetIndex),
    ConfigureCharset(Charset, CharsetIndex),
    /// Cursor specific actions
    ///
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
    /// Keyboard specific actions
    ///
    SetKeypadApplicationMode,
    UnsetKeypadApplicationMode,
    StoreToClipboard(ClipboardType, Vec<u8>),
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

pub trait Actor {
    fn handle(&mut self, _: Action) {}
}
