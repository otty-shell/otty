use std::collections::VecDeque;

use log::debug;

use crate::escape::{Action, EscapeActor};
use crate::surface::SurfaceActor;
use crate::terminal::{SyncState, TerminalClient, TerminalEvent};

pub(super) struct TerminalSurfaceActor<'a, S> {
    pub surface: &'a mut S,
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
            NewLine => self.surface.new_line(),
            NextLine => {
                self.surface.line_feed();
                self.surface.carriage_return();
            },
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
                self.surface.identify_terminal(response, self.pending_input);
            },
            ReportDeviceStatus(status) => {
                self.surface
                    .report_device_status(status, self.pending_input);
            },
            SetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(true);
            },
            UnsetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(false);
            },
            ReportKeyboardMode => {
                self.surface.report_keyboard_mode(self.pending_input)
            },
            SetKeyboardMode(mode, behavior) => {
                self.surface.set_keyboard_mode(mode, behavior);
            },
            PushKeyboardMode(mode) => {
                self.surface.push_keyboard_mode(mode);
            },
            PopKeyboardModes(amount) => {
                self.surface.pop_keyboard_modes(amount);
            },
            SetMode(mode) => self.surface.set_mode(mode),
            SetPrivateMode(mode) => {
                self.surface.set_private_mode(mode);
            },
            UnsetMode(mode) => {
                self.surface.unset_mode(mode);
            },
            UnsetPrivateMode(mode) => {
                self.surface.unset_private_mode(mode);
            },
            ReportMode(mode) => {
                self.surface.report_mode(mode, self.pending_input)
            },
            ReportPrivateMode(mode) => {
                self.surface.report_private_mode(mode, self.pending_input)
            },
            PushWindowTitle => self.surface.push_window_title(),
            PopWindowTitle => {
                if let Some(title) = self.surface.pop_window_title() {
                    self.dispatch_event(TerminalEvent::TitleChanged { title });
                } else {
                    self.dispatch_event(TerminalEvent::ResetTitle);
                }
            },
            SetWindowTitle(title) => {
                self.surface.set_window_title(Some(title.clone()));
                self.dispatch_event(TerminalEvent::TitleChanged { title });
            },
            action => debug!("unsupported action: {action:?}"),
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
        self.sync_state.begin();
    }

    fn end_sync(&mut self) {
        if !self.sync_state.is_active() {
            return;
        }
        let actions = self.sync_state.end();
        self.flush_buffered_actions(actions);
    }
}
