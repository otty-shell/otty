use std::collections::VecDeque;

use log::trace;

use crate::escape::{
    Action, EscapeActor, KeyboardMode, KeyboardModeApplyBehavior, Mode,
    NamedMode, NamedPrivateMode, PrivateMode,
};
use crate::terminal::mode::TerminalMode;
use crate::terminal::actor::SurfaceActor;
use crate::terminal::{TerminalClient, TerminalEvent, SyncState};

pub(super) struct TerminalSurfaceActor<'a, S> {
    pub surface: &'a mut S,
    pub mode: &'a mut TerminalMode,
    pub keyboard_mode: &'a mut KeyboardMode,
    pub keyboard_stack: &'a mut Vec<KeyboardMode>,
    pub client: &'a mut Option<Box<dyn TerminalClient + 'static>>,
    pub pending_input: &'a mut VecDeque<u8>,
    pub sync_state: &'a mut SyncState,
}

impl<'a, S: SurfaceActor> TerminalSurfaceActor<'a, S> {
    fn dispatch_event(&mut self, event: TerminalEvent) {
        if let Some(client) = self.client {
            let _ = client.handle_event(event);
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        use NamedMode::*;

        if let Mode::Named(named) = mode {
            match named {
                Insert => {
                    self.mode.insert(TerminalMode::INSERT);
                },
                LineFeedNewLine => {
                    self.mode.insert(TerminalMode::LINE_FEED_NEW_LINE);
                },
            }
        }
    }

    fn unset_mode(&mut self, mode: Mode) {
        use NamedMode::*;

        if let Mode::Named(named) = mode {
            match named {
                Insert => {
                    self.mode.remove(TerminalMode::INSERT);
                },
                LineFeedNewLine => {
                    self.mode.remove(TerminalMode::LINE_FEED_NEW_LINE);
                },
            }
        }
    }

    fn report_mode(&mut self, mode: Mode) {
        use NamedMode::*;

        let state = match mode {
            Mode::Named(mode) => match mode {
                Insert => self.mode.contains(TerminalMode::INSERT).into(),
                LineFeedNewLine => {
                    self.mode.contains(TerminalMode::LINE_FEED_NEW_LINE).into()
                },
            },
            Mode::Unknown(_) => 0,
        };

        self.pending_input.extend(
            format!("\x1b[{};{}$y", mode.raw(), state as u8,).as_bytes(),
        );
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        use NamedPrivateMode::*;

        if let PrivateMode::Named(named) = mode {
            match named {
                CursorKeys => self.mode.insert(TerminalMode::APP_CURSOR),
                Origin => {
                    self.mode.insert(TerminalMode::ORIGIN);
                    self.surface.goto(0, 0);
                },
                LineWrap => {
                    self.mode.insert(TerminalMode::LINE_WRAP);
                },
                ShowCursor => self.mode.insert(TerminalMode::SHOW_CURSOR),
                ReportMouseClicks => {
                    self.mode.remove(TerminalMode::MOUSE_MODE);
                    self.mode.insert(TerminalMode::MOUSE_REPORT_CLICK);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportCellMouseMotion => {
                    self.mode.remove(TerminalMode::MOUSE_MODE);
                    self.mode.insert(TerminalMode::MOUSE_DRAG);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportAllMouseMotion => {
                    self.mode.remove(TerminalMode::MOUSE_MODE);
                    self.mode.insert(TerminalMode::MOUSE_MOTION);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportFocusInOut => {
                    self.mode.insert(TerminalMode::FOCUS_IN_OUT)
                },
                Utf8Mouse => {
                    self.mode.remove(TerminalMode::SGR_MOUSE);
                    self.mode.insert(TerminalMode::UTF8_MOUSE);
                },
                SgrMouse => {
                    self.mode.remove(TerminalMode::UTF8_MOUSE);
                    self.mode.insert(TerminalMode::SGR_MOUSE);
                },
                AlternateScroll => {
                    self.mode.insert(TerminalMode::ALTERNATE_SCROLL)
                },
                UrgencyHints => self.mode.insert(TerminalMode::URGENCY_HINTS),
                SwapScreenAndSetRestoreCursor => {
                    if !self.mode.contains(TerminalMode::ALT_SCREEN) {
                        self.surface.swap_altscreen(true);
                        self.mode.insert(TerminalMode::ALT_SCREEN)
                    }
                },
                BracketedPaste => {
                    self.mode.insert(TerminalMode::BRACKETED_PASTE)
                },
                ColumnMode => self.surface.deccolm(),
                _ => {},
            }
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        use NamedPrivateMode::*;

        if let PrivateMode::Named(named) = mode {
            match named {
                CursorKeys => self.mode.remove(TerminalMode::APP_CURSOR),
                Origin => {
                    self.mode.remove(TerminalMode::ORIGIN);
                },
                LineWrap => {
                    self.mode.remove(TerminalMode::LINE_WRAP);
                },
                ShowCursor => self.mode.remove(TerminalMode::SHOW_CURSOR),
                ReportMouseClicks => {
                    self.mode.remove(TerminalMode::MOUSE_REPORT_CLICK);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportCellMouseMotion => {
                    self.mode.remove(TerminalMode::MOUSE_DRAG);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportAllMouseMotion => {
                    self.mode.remove(TerminalMode::MOUSE_MOTION);
                    // TODO: send event
                    // self.event_proxy.send_event(Event::MouseCursorDirty);
                },
                ReportFocusInOut => {
                    self.mode.remove(TerminalMode::FOCUS_IN_OUT)
                },
                Utf8Mouse => {
                    self.mode.remove(TerminalMode::UTF8_MOUSE);
                },
                SgrMouse => {
                    self.mode.remove(TerminalMode::SGR_MOUSE);
                },
                AlternateScroll => {
                    self.mode.remove(TerminalMode::ALTERNATE_SCROLL)
                },
                UrgencyHints => self.mode.remove(TerminalMode::URGENCY_HINTS),
                SwapScreenAndSetRestoreCursor => {
                    if self.mode.contains(TerminalMode::ALT_SCREEN) {
                        self.surface.swap_altscreen(false);
                        self.mode.remove(TerminalMode::ALT_SCREEN)
                    }
                },
                BracketedPaste => {
                    self.mode.remove(TerminalMode::BRACKETED_PASTE)
                },
                ColumnMode => self.surface.deccolm(),
                _ => {},
            }
        }
    }

    fn report_private_mode(&mut self, mode: PrivateMode) {
        use NamedPrivateMode::*;

        let state = match mode {
            PrivateMode::Named(mode) => match mode {
                CursorKeys => {
                    self.mode.contains(TerminalMode::APP_CURSOR).into()
                },
                Origin => self.mode.contains(TerminalMode::ORIGIN).into(),
                LineWrap => self.mode.contains(TerminalMode::LINE_WRAP).into(),
                ShowCursor => {
                    self.mode.contains(TerminalMode::SHOW_CURSOR).into()
                },
                ReportMouseClicks => {
                    self.mode.contains(TerminalMode::MOUSE_REPORT_CLICK).into()
                },
                ReportCellMouseMotion => {
                    self.mode.contains(TerminalMode::MOUSE_DRAG).into()
                },
                ReportAllMouseMotion => {
                    self.mode.contains(TerminalMode::MOUSE_MOTION).into()
                },
                ReportFocusInOut => {
                    self.mode.contains(TerminalMode::FOCUS_IN_OUT).into()
                },
                Utf8Mouse => {
                    self.mode.contains(TerminalMode::UTF8_MOUSE).into()
                },
                SgrMouse => self.mode.contains(TerminalMode::SGR_MOUSE).into(),
                AlternateScroll => {
                    self.mode.contains(TerminalMode::ALTERNATE_SCROLL).into()
                },
                UrgencyHints => {
                    self.mode.contains(TerminalMode::URGENCY_HINTS).into()
                },
                SwapScreenAndSetRestoreCursor => {
                    self.mode.contains(TerminalMode::ALT_SCREEN).into()
                },
                BracketedPaste => {
                    self.mode.contains(TerminalMode::BRACKETED_PASTE).into()
                },
                SyncUpdate => 2,
                ColumnMode => 0,
                _ => 0,
            },
            PrivateMode::Unknown(_) => 0,
        };

        self.pending_input.extend(
            format!("\x1b[?{};{}$y", mode.raw(), state as u8,).as_bytes(),
        );
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
        self.surface.push_keyboard_mode();
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
        self.surface.pop_keyboard_modes(amount);
    }

    fn sync_keyboard_flags(&mut self) {
        if self.keyboard_mode.is_empty() {
            self.mode.remove(TerminalMode::KITTY_KEYBOARD_PROTOCOL);
        } else {
            self.mode.insert(TerminalMode::KITTY_KEYBOARD_PROTOCOL);
        }
    }

    fn process_action(&mut self, action: Action) {
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
            SetTabs(mask) => self.surface.insert_tabs(mask as usize),
            ClearTabs(mode) => self.surface.clear_tabs(mode),
            ScreenAlignmentDisplay => self.surface.screen_alignment_display(),
            MoveForwardTabs(count) => {
                self.surface.move_forward_tabs(count as usize)
            },
            MoveBackwardTabs(count) => {
                self.surface.move_backward_tabs(count as usize)
            },
            SetActiveCharsetIndex(index) => {
                self.surface.set_active_charset_index(index);
            },
            ConfigureCharset(charset, index) => {
                self.surface.configure_charset(charset, index);
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
                self.mode.insert(TerminalMode::APP_KEYPAD);
            },
            UnsetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(false);
                self.mode.remove(TerminalMode::APP_KEYPAD);
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
            },
            PopKeyboardModes(amount) => {
                self.pop_keyboard_modes(amount);
            },
            SetMode(mode) => self.set_mode(mode),
            SetPrivateMode(mode) => {
                self.set_private_mode(mode);
            },
            UnsetMode(mode) => {
                self.unset_mode(mode);
            },
            UnsetPrivateMode(mode) => {
                self.unset_private_mode(mode);
            },
            ReportMode(mode) => self.report_mode(mode),
            ReportPrivateMode(mode) => self.report_private_mode(mode),
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

    fn flush_buffered_actions(&mut self, actions: Vec<Action>) {
        for action in actions {
            self.process_action(action);
        }
    }

    fn abort_sync(&mut self) -> bool {
        if !self.sync_state.is_active() {
            return false;
        }
        let actions = self.sync_state.cancel();
        self.surface.end_sync();
        self.flush_buffered_actions(actions);
        true
    }

    pub(super) fn flush_sync_timeout(&mut self) -> bool {
        if self.sync_state.is_expired() {
            if self.sync_state.is_active() {
                return self.abort_sync();
            }
            self.sync_state.refresh_deadline();
        }
        false
    }
}

impl<'a, S: SurfaceActor> EscapeActor for TerminalSurfaceActor<'a, S> {
    fn handle(&mut self, action: Action) {
        if self.sync_state.is_active() {
            if self.sync_state.is_expired() {
                self.abort_sync();
            }

            if self.sync_state.is_active() {
                match self.sync_state.push(action) {
                    Ok(()) => return,
                    Err(overflowed) => {
                        let _ = self.abort_sync();
                        self.process_action(overflowed);
                        return;
                    },
                }
            }
        }

        self.process_action(action);
    }

    fn begin_sync(&mut self) {
        if self.sync_state.is_active() {
            return;
        }
        self.surface.begin_sync();
        self.sync_state.begin();
    }

    fn end_sync(&mut self) {
        if !self.sync_state.is_active() {
            return;
        }
        let actions = self.sync_state.end();
        self.surface.end_sync();
        self.flush_buffered_actions(actions);
    }
}
