use cursor_icon::CursorIcon;

use crate::{
    CharacterAttribute, Charset, CharsetIndex, ClearMode, CursorShape,
    CursorStyle, Hyperlink, LineClearMode, Mode, PrivateMode, Rgb,
    TabClearMode,
    keyboard::{KeyboardMode, KeyboardModeApplyBehavior, ModifyOtherKeysState},
};

#[derive(Debug, PartialEq, Eq)]
/// High level actions emitted by the escape sequence parser for the terminal
/// emulator to execute.
pub enum Action {
    /// Render the provided character at the cursor position.
    Print(char),
    /// Play the terminal bell / alert sound.
    Bell,
    /// Insert the requested number of blank cells at the cursor position.
    InsertBlank(usize),
    /// Insert blank lines at the cursor row, pushing existing lines down.
    InsertBlankLines(usize),
    /// Delete lines starting at the cursor row, pulling lines from below.
    DeleteLines(usize),
    /// Remove characters from the cursor position and shift the row left.
    DeleteChars(usize),
    /// Overwrite a span of characters from the cursor with blanks.
    EraseChars(usize),
    /// Move the cursor one column to the left, erasing if necessary.
    Backspace,
    /// Move the cursor to the start of the current line.
    CarriageReturn,
    /// Advance the cursor downward, potentially scrolling.
    LineFeed,
    /// Move the cursor to the next line without altering horizontal position.
    NextLine,
    /// Move down and reset to column zero (common newline behavior).
    NewLine,
    /// Replace the current character with the replacement character (SUB).
    Substitute,
    /// Set a horizontal tab stop at the current cursor column.
    SetHorizontalTab,
    /// Move up one line, scrolling the viewport if already at the top.
    ReverseIndex,
    /// Reset terminal state to the parser defaults.
    ResetState,
    /// Clear screen content according to the specified clear mode.
    ClearScreen(ClearMode),
    /// Clear part or all of the active line per the clear mode.
    ClearLine(LineClearMode),
    /// Insert the given number of horizontal tabs at the cursor.
    InsertTabs(u16),
    /// Set tab stops using the provided bit-field mask.
    SetTabs(u16),
    /// Clear horizontal tabs according to the tab clear mode.
    ClearTabs(TabClearMode),
    /// Enable the DEC screen alignment test (fills display with 'E').
    ScreenAlignmentDisplay,
    /// Move the cursor forward by the given number of tab stops.
    MoveForwardTabs(u16),
    /// Move the cursor backward by the given number of tab stops.
    MoveBackwardTabs(u16),
    /// Select which charset slot is active for subsequent character writes.
    SetActiveCharsetIndex(CharsetIndex),
    /// Load a charset definition into the specified slot.
    ConfigureCharset(Charset, CharsetIndex),
    /// Update an indexed color in the terminal palette.
    SetColor { index: usize, color: Rgb },
    /// Request the current value of an indexed palette color.
    QueryColor(usize),
    /// Reset an indexed color back to its default value.
    ResetColor(usize),
    /// Define the inclusive top and bottom rows for scroll operations.
    SetScrollingRegion(usize, usize),
    /// Scroll the active region up by the specified amount.
    ScrollUp(usize),
    /// Scroll the active region down by the specified amount.
    ScrollDown(usize),
    /// Set or clear the active hyperlink for subsequent text.
    SetHyperlink(Option<Hyperlink>),
    /// Apply Select Graphic Rendition attributes (colors, style, etc.).
    SGR(CharacterAttribute),
    /// Set the text cursor shape.
    SetCursorShape(CursorShape),
    /// Set the cursor pointer type (https://www.w3.org/TR/css-ui-3/#cursor).
    SetCursorIcon(CursorIcon),
    /// Set the cursor rendering style (blinking, steady, block, etc.).
    SetCursorStyle(Option<CursorStyle>),
    /// Save the current cursor position, charset state, and modes.
    SaveCursorPosition,
    /// Restore a previously saved cursor position and related state.
    RestoreCursorPosition,
    /// Cursor: move up by rows, with optional carriage return.
    MoveUp {
        rows: usize,
        carrage_return_needed: bool,
    },
    /// Cursor: move down by rows, with optional carriage return.
    MoveDown {
        rows: usize,
        carrage_return_needed: bool,
    },
    /// Cursor: move forward (right) by the given number of columns.
    MoveForward(usize),
    /// Cursor: move backward (left) by the given number of columns.
    MoveBackward(usize),
    /// Cursor: go to the provided row and column.
    Goto(i32, usize),
    /// Cursor: reposition to the specified row while keeping column.
    GotoRow(i32),
    /// Cursor: reposition to the specified column on the current row.
    GotoColumn(usize),
    /// Reply to a terminal identity request (DECID / DA).
    IdentifyTerminal(Option<char>),
    /// Reply to a general device status report (DSR).
    ReportDeviceStatus(usize),
    /// Keyboard: switch to the application keypad mode.
    SetKeypadApplicationMode,
    /// Keyboard: switch back to the numeric keypad mode.
    UnsetKeypadApplicationMode,
    /// Keyboard: enable or configure modifyOtherKeys state.
    SetModifyOtherKeysState(ModifyOtherKeysState),
    /// Keyboard: report the current modifyOtherKeys state.
    ReportModifyOtherKeysState,
    /// Keyboard: report the current keyboard protocol mode.
    ReportKeyboardMode,
    /// Keyboard: set the active keyboard protocol and behavior.
    SetKeyboardMode(KeyboardMode, KeyboardModeApplyBehavior),
    /// Keyboard: push a keyboard mode onto the stack.
    PushKeyboardMode(KeyboardMode),
    /// Keyboard: pop keyboard modes from the stack.
    PopKeyboardModes(u16),
    /// Mode: enable a DEC/ANSI line discipline mode.
    SetMode(Mode),
    /// Mode: enable a DEC private mode.
    SetPrivateMode(PrivateMode),
    /// Mode: disable a DEC/ANSI line discipline mode.
    UnsetMode(Mode),
    /// Mode: disable a DEC private mode.
    UnsetPrivateMode(PrivateMode),
    /// Mode: report the current state of a DEC/ANSI mode.
    ReportMode(Mode),
    /// Mode: report the current state of a DEC private mode.
    ReportPrivateMode(PrivateMode),
    /// Window: request pixel-based viewport dimensions.
    RequestTextAreaSizeByPixels,
    /// Window: request character-cell viewport dimensions.
    RequestTextAreaSizeByChars,
    /// Window: push the current window title onto the stack.
    PushWindowTitle,
    /// Window: restore the last pushed window title.
    PopWindowTitle,
    /// Window: set the terminal window title.
    SetWindowTitle(String),
}

pub trait EscapeActor {
    /// processing the action
    fn handle(&mut self, _: Action) {}
    /// Begin synchronized (batch) update (DEC mode 2026)
    fn begin_sync(&mut self) {}
    /// End synchronized (batch) update (DEC mode 2026)
    fn end_sync(&mut self) {}
}
