use std::collections::VecDeque;
use std::io::ErrorKind;
use std::process::ExitStatus;

use log::trace;
use mio::Registry;
use mio::{Interest, Token};

use crate::TerminalMode;
use crate::error::Result;
use crate::escape::{Action, EscapeActor, EscapeParser};
use crate::options::TerminalOptions;
use crate::pty::{Pollable, PtySize, Session, SessionError};
use crate::runtime::{
    IoHandler, LifecycleControl, PendingOutput, RequestHandler,
    RuntimeMaintenance, SessionRegistration, TerminalClient, TerminalEvent,
    TerminalRequest,
};
use crate::surface::SurfaceController;

const DEFAULT_READ_BUFFER_CAPACITY: usize = 1024;

type DynTerminalClient<S> = dyn TerminalClient<S> + 'static;

/// High level runtime that connects a PTY session with the escape parser and
/// in-memory surface model.
pub struct Terminal<P, E, S> {
    session: P,
    parser: E,
    surface: S,
    read_buffer: Vec<u8>,
    exit_status: Option<ExitStatus>,
    mode: TerminalMode,
    pending_input: VecDeque<u8>,
    event_client: Option<Box<DynTerminalClient<S>>>,
}

struct TerminalSurfaceActor<'a, S> {
    surface: &'a mut S,
    mode: &'a mut TerminalMode,
}

impl<'a, S: SurfaceController> EscapeActor for TerminalSurfaceActor<'a, S> {
    fn handle(&mut self, action: Action) {
        use Action::*;

        match action {
            Print(ch) => self.surface.print(ch),
            Bell => self.surface.bell(),
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
            Substitute => self.surface.print('�'),
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
            SetHyperlink(link) => self.surface.set_hyperlink(link),
            SGR(attribute) => self.surface.sgr(attribute),
            SetCursorShape(shape) => self.surface.set_cursor_shape(shape),
            SetCursorIcon(icon) => self.surface.set_cursor_icon(icon),
            SetCursorStyle(style) => self.surface.set_cursor_style(style),
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
                self.surface.set_keypad_application_mode(true)
            },
            UnsetKeypadApplicationMode => {
                self.surface.set_keypad_application_mode(false)
            },
            SetModifyOtherKeysState(state) => {
                trace!("modifyOtherKeys => {:?}", state);
            },
            ReportModifyOtherKeysState => trace!("Report modifyOtherKeys"),
            ReportKeyboardMode => trace!("Report keyboard mode"),
            SetKeyboardMode(mode, behavior) => {
                trace!("Set keyboard mode {:?} {:?}", mode, behavior);
            },
            PushKeyboardMode(_) => self.surface.push_keyboard_mode(),
            PopKeyboardModes(amount) => self.surface.pop_keyboard_modes(amount),
            SetMode(mode) => self.surface.set_mode(mode, true),
            SetPrivateMode(mode) => self.surface.set_private_mode(mode, true),
            UnsetMode(mode) => self.surface.set_mode(mode, false),
            UnsetPrivateMode(mode) => {
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
            SetWindowTitle(title) => self.surface.set_window_title(title),
        }
    }

    fn begin_sync(&mut self) {}

    fn end_sync(&mut self) {}
}

impl<P, E, S> Terminal<P, E, S>
where
    P: Session,
    E: EscapeParser,
    S: SurfaceController,
{
    pub fn new(
        session: P,
        surface: S,
        parser: E,
        options: TerminalOptions,
    ) -> Result<Self> {
        let mut read_buffer = vec![
            0u8;
            options
                .read_buffer_capacity
                .max(DEFAULT_READ_BUFFER_CAPACITY)
        ];
        if read_buffer.is_empty() {
            read_buffer.resize(DEFAULT_READ_BUFFER_CAPACITY, 0);
        }

        Ok(Self {
            session,
            parser,
            surface,
            read_buffer,
            exit_status: None,
            mode: TerminalMode::default(),
            pending_input: VecDeque::new(),
            event_client: None,
        })
    }

    /// Attach a client that will receive [`TerminalEvent`] callbacks directly from this terminal.
    pub fn set_event_client<C>(&mut self, client: C)
    where
        C: TerminalClient<S> + 'static,
    {
        self.event_client = Some(Box::new(client));
    }

    /// Remove the currently attached event client.
    pub fn take_event_client(&mut self) -> Option<Box<DynTerminalClient<S>>> {
        self.event_client.take()
    }

    /// Check whether the terminal has an event client attached.
    pub fn has_event_client(&self) -> bool {
        self.event_client.is_some()
    }

    /// Apply a [`TerminalRequest`] without going through the [`Runtime`].
    pub fn apply_request(&mut self, request: TerminalRequest) -> Result<()> {
        self.process_request(request)
    }

    /// Drain any readable data from the PTY, returning whether the surface changed.
    pub fn poll_output(&mut self) -> Result<bool> {
        self.drain_session()
    }

    /// Poll the child process for exit status updates.
    pub fn poll_exit(&mut self) -> Result<Option<ExitStatus>> {
        self.capture_exit()
    }

    /// Write a chunk of bytes to the PTY session.
    fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        let mut written = 0usize;

        while written < bytes.len() {
            match self.session.write(&bytes[written..]) {
                Ok(0) => break,
                Ok(count) => written += count,
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    break;
                },
                Err(err) => return Err(err.into()),
            }
        }

        Ok(written)
    }

    /// Request a PTY resize and mirror the new geometry in the surface model.
    fn resize(&mut self, size: PtySize) -> Result<()> {
        self.session.resize(size)?;
        self.surface.resize(size.cols as usize, size.rows as usize);
        self.emit_surface_change()
    }

    /// Terminate the session and return the reported exit status code.
    fn close(&mut self) -> Result<i32> {
        let code = self.session.close()?;
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            self.exit_status = Some(ExitStatusExt::from_raw(code));
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            self.exit_status = Some(ExitStatusExt::from_raw(code as u32));
        }
        if let Some(status) = self.exit_status.as_ref().copied() {
            self.emit_child_exit(&status)?;
        }
        Ok(code)
    }

    /// Borrow the underlying surface controller.
    pub fn surface_actor(&self) -> &S {
        &self.surface
    }

    /// Mutably borrow the surface controller.
    pub fn surface_actor_mut(&mut self) -> &mut S {
        &mut self.surface
    }

    fn capture_exit(&mut self) -> Result<Option<ExitStatus>> {
        match self.session.try_get_child_exit_status() {
            Ok(Some(status)) => {
                self.exit_status = Some(status);
                if let Some(exit_status) = self.exit_status.as_ref().copied() {
                    self.emit_child_exit(&exit_status)?;
                }
                Ok(self.exit_status)
            },
            Ok(None) => Ok(None),
            Err(SessionError::IO(err))
                if err.kind() == ErrorKind::WouldBlock =>
            {
                Ok(None)
            },
            Err(SessionError::IO(err))
                if err.kind() == ErrorKind::Interrupted =>
            {
                Ok(None)
            },
            Err(err) => Err(err.into()),
        }
    }

    fn enqueue_input(&mut self, data: Vec<u8>) {
        if data.is_empty() {
            return;
        }
        self.pending_input.extend(data);
    }

    fn flush_pending_input(&mut self) -> Result<()> {
        while !self.pending_input.is_empty() {
            let chunk = {
                let slice = self.pending_input.make_contiguous();
                slice.to_vec()
            };

            if chunk.is_empty() {
                break;
            }

            let total = chunk.len();
            let written = self.write(&chunk)?;

            if written == 0 {
                break;
            }

            self.pending_input
                .drain(0..written.min(self.pending_input.len()));

            if written < total {
                break;
            }
        }

        Ok(())
    }

    fn emit_surface_change(&mut self) -> Result<()> {
        if let Some(mut client) = self.event_client.take() {
            {
                let surface = self.surface_actor();
                client
                    .handle_event(TerminalEvent::SurfaceChanged { surface })?;
            }
            self.event_client = Some(client);
        }
        Ok(())
    }

    fn emit_child_exit(&mut self, status: &ExitStatus) -> Result<()> {
        if let Some(mut client) = self.event_client.take() {
            client.handle_event(TerminalEvent::ChildExit { status })?;
            self.event_client = Some(client);
        }

        Ok(())
    }

    fn process_request(&mut self, request: TerminalRequest) -> Result<()> {
        match request {
            TerminalRequest::Write(bytes) => {
                self.enqueue_input(bytes);
                self.flush_pending_input()?;
            },
            TerminalRequest::Resize(size) => self.resize(size)?,
            TerminalRequest::Shutdown => {
                self.close()?;
            },
        }

        Ok(())
    }

    fn drain_session(&mut self) -> Result<bool> {
        let mut updated = false;

        loop {
            match self.session.read(self.read_buffer.as_mut_slice()) {
                Ok(0) => break,
                Ok(count) => {
                    let chunk = &self.read_buffer[..count];
                    let parser = &mut self.parser;
                    let surface = &mut self.surface;
                    let mode = &mut self.mode;
                    let mut actor = TerminalSurfaceActor { surface, mode };
                    parser.advance(chunk, &mut actor);
                    updated = true;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::Interrupted =>
                {
                    continue;
                },
                Err(SessionError::IO(err))
                    if err.kind() == ErrorKind::WouldBlock =>
                {
                    break;
                },
                Err(err) => return Err(err.into()),
            }
        }

        if updated {
            self.emit_surface_change()?;
        }

        Ok(updated)
    }
}

impl<P, E, S> SessionRegistration for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn register_session(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()> {
        self.session
            .register(registry, interest, io_token, child_token)?;
        Ok(())
    }

    fn reregister_session(
        &mut self,
        registry: &Registry,
        interest: Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<()> {
        self.session
            .reregister(registry, interest, io_token, child_token)?;
        Ok(())
    }

    fn deregister_session(&mut self, registry: &Registry) -> Result<()> {
        self.session.deregister(registry)?;
        Ok(())
    }
}

impl<P, E, S> IoHandler for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn handle_read_ready(&mut self) -> Result<bool> {
        self.drain_session()
    }

    fn handle_write_ready(&mut self) -> Result<()> {
        self.flush_pending_input()
    }
}

impl<P, E, S> RequestHandler for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn handle_request(&mut self, request: TerminalRequest) -> Result<()> {
        self.process_request(request)
    }
}

impl<P, E, S> LifecycleControl for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        self.capture_exit()
    }
}

impl<P, E, S> RuntimeMaintenance for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn maintain(&mut self) -> Result<()> {
        self.flush_pending_input()
    }
}

impl<P, E, S> PendingOutput for Terminal<P, E, S>
where
    P: Session + Pollable,
    E: EscapeParser,
    S: SurfaceController,
{
    fn has_pending_output(&self) -> bool {
        !self.pending_input.is_empty()
    }
}
