use std::io::ErrorKind;
use std::process::ExitStatus;
use std::time::Duration;

use log::trace;
use mio::Registry;
use mio::{Interest, Token};

use crate::TerminalMode;
use crate::error::Result;
use crate::escape::{Action, EscapeActor, EscapeParser};
use crate::event_loop::{
    TerminalClient, TerminalEventLoop, TerminalLoopTarget,
};
use crate::options::TerminalOptions;
use crate::pty::{Pollable, PtySize, Session, SessionError};
use crate::surface::SurfaceController;

pub trait PtySession: Session + Pollable {}
impl<T> PtySession for T where T: Session + Pollable {}

/// High level runtime that connects a PTY session with the escape parser and
/// in-memory surface model.
pub struct Terminal<P, E, S> {
    session: P,
    parser: E,
    surface: S,
    read_buffer: Vec<u8>,
    options: TerminalOptions,
    exit_status: Option<ExitStatus>,
    running: bool,
    mode: TerminalMode,
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
    P: PtySession,
    E: EscapeParser,
    S: SurfaceController,
{
    pub fn new(
        session: P,
        surface: S,
        parser: E,
        options: TerminalOptions,
    ) -> Result<Self> {
        let mut read_buffer = vec![0u8; options.read_buffer_capacity.max(1024)];

        if read_buffer.is_empty() {
            read_buffer.resize(1024, 0);
        }

        Ok(Self {
            session,
            parser,
            surface,
            read_buffer,
            options,
            exit_status: None,
            running: true,
            mode: TerminalMode::default(),
        })
    }

    /// Run the terminal event loop, delegating front-end duties to the provided client.
    pub fn run<C>(&mut self, client: &mut C) -> Result<()>
    where
        C: TerminalClient<Self> + ?Sized,
    {
        let mut event_loop = TerminalEventLoop::new()?;
        event_loop.run(self, client)
    }

    /// Write a chunk of bytes to the PTY session.
    pub fn write(&mut self, bytes: &[u8]) -> Result<usize> {
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
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        self.session.resize(size)?;
        self.surface.resize(size.cols as usize, size.rows as usize);
        Ok(())
    }

    /// Terminate the session and return the reported exit status code.
    pub fn close(&mut self) -> Result<i32> {
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
        self.running = false;
        Ok(code)
    }

    // /// Access the active surface for inspection or rendering.
    // pub fn surface(&self) -> &Surface {
    //     self.surface
    // }

    // /// Mutably access the surface.
    // pub fn surface_mut(&mut self) -> &mut Surface {
    //     self.surface
    // }

    /// Borrow the underlying surface actor.
    pub fn surface_actor(&self) -> &S {
        &self.surface
    }

    /// Mutably borrow the underlying surface actor.
    pub fn surface_actor_mut(&mut self) -> &mut S {
        &mut self.surface
    }

    /// Check whether the child process is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Retrieve the cached exit status if the child process has terminated.
    pub fn exit_status(&self) -> Option<&ExitStatus> {
        self.exit_status.as_ref()
    }

    fn drain_pty(&mut self) -> Result<bool> {
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

        Ok(updated)
    }

    fn capture_exit(&mut self) -> Result<Option<ExitStatus>> {
        match self.session.try_get_child_exit_status() {
            Ok(Some(status)) => {
                self.exit_status = Some(status);
                self.running = false;
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
}

impl<P, E, S> TerminalLoopTarget for Terminal<P, E, S>
where
    P: PtySession,
    E: EscapeParser,
    S: SurfaceController,
{
    type SurfaceHandle = S;

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

    fn handle_read_ready(&mut self) -> Result<bool> {
        self.drain_pty()
    }

    fn check_child_exit(&mut self) -> Result<Option<ExitStatus>> {
        self.capture_exit()
    }

    fn poll_timeout(&self) -> Option<Duration> {
        Some(self.options.poll_timeout)
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn surface_handle(&self) -> &Self::SurfaceHandle {
        self.surface_actor()
    }

    fn exit_status(&self) -> Option<&ExitStatus> {
        self.exit_status()
    }
}

// #[cfg(test)]
// mod tests {
//     use otty_escape::{Action, CharacterAttribute, Color, StdColor};

//     use super::*;

//     #[test]
//     fn default_surface_actor_handles_print() {
//         let mut actor = DefaultTerminalSurface::default();

//         actor.handle(Action::Print('X'));

//         let grid = actor.surface().grid();
//         assert_eq!(grid.row(0).cells[0].ch, 'X');
//     }

//     #[test]
//     fn default_surface_actor_applies_sgr() {
//         let mut actor = DefaultTerminalSurface::default();

//         actor.handle(Action::SGR(CharacterAttribute::Foreground(Color::Std(
//             StdColor::Blue,
//         ))));
//         actor.handle(Action::Print('A'));

//         let cell = &actor.surface().grid().row(0).cells[0];
//         assert_eq!(cell.ch, 'A');
//         assert_eq!(cell.attributes.foreground, Color::Std(StdColor::Blue));
//     }
// }
