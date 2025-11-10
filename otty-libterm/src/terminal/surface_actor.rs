use log::trace;

use crate::TerminalMode;
use crate::escape::{
    Action, EscapeActor, KeyboardMode, KeyboardModeApplyBehavior, Mode,
    NamedMode, NamedPrivateMode, PrivateMode,
};
use crate::surface::SurfaceController;
use crate::terminal::{TerminalClient, TerminalEvent};

pub(super) struct TerminalSurfaceActor<'a, S> {
    pub surface: &'a mut S,
    pub mode: &'a mut TerminalMode,
    pub keyboard_mode: &'a mut KeyboardMode,
    pub keyboard_stack: &'a mut Vec<KeyboardMode>,
    pub client: &'a mut Option<Box<dyn TerminalClient + 'static>>,
}

impl<'a, S> TerminalSurfaceActor<'a, S> {
    fn dispatch_event(&mut self, event: TerminalEvent) {
        if let Some(client) = self.client {
            let _ = client.handle_event(event);
        }
    }

    fn set_flag(&mut self, flag: TerminalMode, enabled: bool) {
        if enabled {
            self.mode.insert(flag);
        } else {
            self.mode.remove(flag);
        }
    }

    fn update_mode(&mut self, mode: &Mode, enabled: bool) {
        if let Mode::Named(named) = mode {
            match named {
                NamedMode::Insert => {
                    self.set_flag(TerminalMode::INSERT, enabled);
                },
                NamedMode::LineFeedNewLine => {
                    self.set_flag(TerminalMode::LINE_FEED_NEW_LINE, enabled);
                },
            }
        }
    }

    fn update_private_mode(&mut self, mode: &PrivateMode, enabled: bool) {
        if let PrivateMode::Named(named) = mode {
            match named {
                NamedPrivateMode::CursorKeys => {
                    self.set_flag(TerminalMode::APP_CURSOR, enabled);
                },
                NamedPrivateMode::Origin => {
                    self.set_flag(TerminalMode::ORIGIN, enabled);
                },
                NamedPrivateMode::LineWrap => {
                    self.set_flag(TerminalMode::LINE_WRAP, enabled);
                },
                NamedPrivateMode::ShowCursor => {
                    self.set_flag(TerminalMode::SHOW_CURSOR, enabled);
                },
                NamedPrivateMode::ReportMouseClicks => {
                    self.set_flag(TerminalMode::MOUSE_REPORT_CLICK, enabled);
                },
                NamedPrivateMode::ReportCellMouseMotion => {
                    self.set_flag(TerminalMode::MOUSE_DRAG, enabled);
                },
                NamedPrivateMode::ReportAllMouseMotion => {
                    self.set_flag(TerminalMode::MOUSE_MOTION, enabled);
                },
                NamedPrivateMode::ReportFocusInOut => {
                    self.set_flag(TerminalMode::FOCUS_IN_OUT, enabled);
                },
                NamedPrivateMode::Utf8Mouse => {
                    self.set_flag(TerminalMode::UTF8_MOUSE, enabled);
                },
                NamedPrivateMode::SgrMouse => {
                    self.set_flag(TerminalMode::SGR_MOUSE, enabled);
                },
                NamedPrivateMode::AlternateScroll => {
                    self.set_flag(TerminalMode::ALTERNATE_SCROLL, enabled);
                },
                NamedPrivateMode::UrgencyHints => {
                    self.set_flag(TerminalMode::URGENCY_HINTS, enabled);
                },
                NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                    self.set_flag(TerminalMode::ALT_SCREEN, enabled);
                },
                NamedPrivateMode::BracketedPaste => {
                    self.set_flag(TerminalMode::BRACKETED_PASTE, enabled);
                },
                _ => {},
            }
        }
    }

    fn apply_keyboard_mode(
        &mut self,
        mode: KeyboardMode,
        behavior: KeyboardModeApplyBehavior,
    ) {
        match behavior {
            KeyboardModeApplyBehavior::Replace => {
                *self.keyboard_mode = mode;
            },
            KeyboardModeApplyBehavior::Union => {
                self.keyboard_mode.insert(mode);
            },
            KeyboardModeApplyBehavior::Difference => {
                self.keyboard_mode.remove(mode);
            },
        }
        self.sync_keyboard_flags();
    }

    fn push_keyboard_mode(&mut self, mode: KeyboardMode) {
        self.keyboard_stack.push(*self.keyboard_mode);
        *self.keyboard_mode = mode;
        self.sync_keyboard_flags();
    }

    fn pop_keyboard_modes(&mut self, amount: u16) {
        for _ in 0..amount {
            if let Some(prev) = self.keyboard_stack.pop() {
                *self.keyboard_mode = prev;
            } else {
                *self.keyboard_mode = KeyboardMode::default();
                break;
            }
        }
        self.sync_keyboard_flags();
    }

    fn sync_keyboard_flags(&mut self) {
        if self.keyboard_mode.is_empty() {
            self.mode.remove(TerminalMode::KITTY_KEYBOARD_PROTOCOL);
        } else {
            self.mode.insert(TerminalMode::KITTY_KEYBOARD_PROTOCOL);
        }
    }
}

impl<'a, S: SurfaceController> EscapeActor for TerminalSurfaceActor<'a, S> {
    fn handle(&mut self, action: Action) {
        use Action::*;

        match action {
            Print(ch) => self.surface.print(ch),
            Bell => {
                self.dispatch_event(TerminalEvent::Bell);
            },
            InsertBlank(count) => self.surface.insert_blank(count),
            InsertBlankLines(count) => self.surface.insert_blank_lines(count),
            DeleteLines(count) => self.surface.delete_lines(count),
            DeleteChars(count) => self.surface.delete_chars(count),
            EraseChars(count) => self.surface.erase_chars(count),
            Backspace => self.surface.backspace(),
            CarriageReturn => self.surface.carriage_return(),
            LineFeed => self.surface.line_feed(),
            NewLine => {
                self.surface.line_feed();
                if self.mode.contains(TerminalMode::LINE_FEED_NEW_LINE) {
                    self.surface.carriage_return();
                }
            },
            NextLine => {
                self.surface.line_feed();
                self.surface.carriage_return();
            },
            Substitute => {},
            SetHorizontalTab => self.surface.set_horizontal_tab(),
            ReverseIndex => self.surface.reverse_index(),
            ResetState => self.surface.reset(),
            ClearScreen(mode) => self.surface.clear_screen(mode),
            ClearLine(mode) => self.surface.clear_line(mode),
            InsertTabs(count) => self.surface.insert_tabs(count as usize),
            SetTabs(mask) => self.surface.set_tabs(mask),
            ClearTabs(mode) => self.surface.clear_tabs(mode),
            ScreenAlignmentDisplay => self.surface.screen_alignment_display(),
            MoveForwardTabs(count) => {
                self.surface.move_forward_tabs(count as usize)
            },
            MoveBackwardTabs(count) => {
                self.surface.move_backward_tabs(count as usize)
            },
            SetActiveCharsetIndex(_) | ConfigureCharset(_, _) => {
                trace!("Charset handling not implemented yet");
            },
            SetColor { index, color } => self.surface.set_color(index, color),
            QueryColor(index) => self.surface.query_color(index),
            ResetColor(index) => self.surface.reset_color(index),
            SetScrollingRegion(top, bottom) => {
                self.surface.set_scrolling_region(top, bottom);
            },
            ScrollUp(count) => self.surface.scroll_up(count),
            ScrollDown(count) => self.surface.scroll_down(count),
            SetHyperlink(link) => {
                self.surface.set_hyperlink(link.clone());
                self.dispatch_event(TerminalEvent::Hyperlink { link });
            },
            SGR(attribute) => self.surface.sgr(attribute),
            SetCursorShape(shape) => {
                self.surface.set_cursor_shape(shape);
                self.dispatch_event(TerminalEvent::CursorShapeChanged {
                    shape,
                });
            },
            SetCursorIcon(icon) => {
                self.surface.set_cursor_icon(icon);
                self.dispatch_event(TerminalEvent::CursorIconChanged { icon });
            },
            SetCursorStyle(style) => {
                self.surface.set_cursor_style(style);
                self.dispatch_event(TerminalEvent::CursorStyleChanged {
                    style,
                });
            },
            SaveCursorPosition => self.surface.save_cursor(),
            RestoreCursorPosition => self.surface.restore_cursor(),
            MoveUp {
                rows,
                carrage_return_needed,
            } => self.surface.move_up(rows, carrage_return_needed),
            MoveDown {
                rows,
                carrage_return_needed,
            } => self.surface.move_down(rows, carrage_return_needed),
            MoveForward(cols) => self.surface.move_forward(cols),
            MoveBackward(cols) => self.surface.move_backward(cols),
            Goto(row, col) => self.surface.goto(row, col),
            GotoRow(row) => self.surface.goto_row(row),
            GotoColumn(col) => self.surface.goto_column(col),
            IdentifyTerminal(response) => {
                trace!("Identify terminal {:?}", response);
            },
            ReportDeviceStatus(status) => {
                trace!("Report device status {}", status);
            },
            SetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(true);
                self.set_flag(TerminalMode::APP_KEYPAD, true);
            },
            UnsetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(false);
                self.set_flag(TerminalMode::APP_KEYPAD, false);
            },
            SetModifyOtherKeysState(state) => {
                trace!("modifyOtherKeys => {:?}", state);
            },
            ReportModifyOtherKeysState => trace!("Report modifyOtherKeys"),
            ReportKeyboardMode => trace!("Report keyboard mode"),
            SetKeyboardMode(mode, behavior) => {
                self.apply_keyboard_mode(mode, behavior);
            },
            PushKeyboardMode(mode) => {
                self.push_keyboard_mode(mode);
                self.surface.push_keyboard_mode();
            },
            PopKeyboardModes(amount) => {
                self.pop_keyboard_modes(amount);
                self.surface.pop_keyboard_modes(amount);
            },
            SetMode(mode) => {
                self.update_mode(&mode, true);
                self.surface.set_mode(mode, true);
            },
            SetPrivateMode(mode) => {
                self.update_private_mode(&mode, true);
                self.surface.set_private_mode(mode, true);
            },
            UnsetMode(mode) => {
                self.update_mode(&mode, false);
                self.surface.set_mode(mode, false);
            },
            UnsetPrivateMode(mode) => {
                self.update_private_mode(&mode, false);
                self.surface.set_private_mode(mode, false)
            },
            ReportMode(mode) => trace!("Report mode {:?}", mode),
            ReportPrivateMode(mode) => trace!("Report private mode {:?}", mode),
            RequestTextAreaSizeByPixels => {
                trace!("Request text area size (pixels)");
            },
            RequestTextAreaSizeByChars => {
                trace!("Request text area size (chars)");
            },
            PushWindowTitle => self.surface.push_window_title(),
            PopWindowTitle => self.surface.pop_window_title(),
            SetWindowTitle(title) => {
                self.surface.set_window_title(title.clone());
                self.dispatch_event(TerminalEvent::TitleChanged { title });
            },
        }
    }

    fn begin_sync(&mut self) {}

    fn end_sync(&mut self) {}
}
