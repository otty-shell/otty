//! Terminal engine for PTY sessions, escape parsing and surface state.
//!
//! This crate connects the lower-level building blocks from the OTTY
//! workspace:
//! - [`otty_pty`] for spawning and driving PTY or SSH sessions,
//! - [`otty_escape`] for parsing terminal escape sequences into semantic
//!   actions,
//! - [`otty_surface`] for maintaining an in-memory terminal screen model.
//!
//! The main entry points are:
//! - [`TerminalEngine`], which owns a PTY session, escape parser and surface,
//!   and exposes a high-level API (`TerminalRequest` / `TerminalEvent`).
//! - [`Runtime`], a small `mio`-based event loop that remains available as a
//!   low-level driver stub for future tasks.
//!
//! Front-ends usually:
//! 1. Construct a PTY [`pty::Session`], an [`escape::EscapeParser`] instance
//!    and a [`surface::SurfaceActor`] implementation.
//! 2. Wrap them in a [`TerminalEngine`].
//! 3. Drive `on_readable` / `on_writable` / `tick` based on your preferred
//!    readiness model, and drain [`TerminalEvent`]s from [`TerminalEvents`].

mod error;
mod runtime;
mod terminal;

pub use error::{Error, Result};
pub use runtime::{Driver, Runtime, RuntimeHooks, RuntimeRequestProxy};
pub use terminal::{
    SnapshotArc, TerminalEngine, TerminalEvent, TerminalRequest,
    builder::{DefaultParser, DefaultSurface, TerminalBuilder, Terminal, TerminalWithRuntime},
    channel::{
        ChannelConfig, ChannelRecvError, ChannelSendError, ChannelTryRecvError,
        TerminalEvents, TerminalHandle,
    },
    options::TerminalOptions,
    size::TerminalSize,
};

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;

#[cfg(test)]
pub(crate) mod tests {
    use std::{collections::VecDeque, io, process::ExitStatus};

    use super::*;

    #[derive(Default)]
    pub struct StubParser {
        actions: VecDeque<escape::Action>,
    }

    impl StubParser {
        pub fn with_actions(actions: Vec<escape::Action>) -> Self {
            Self {
                actions: actions.into(),
            }
        }
    }

    impl escape::EscapeParser for StubParser {
        fn advance<A: escape::EscapeActor>(
            &mut self,
            _bytes: &[u8],
            actor: &mut A,
        ) {
            while let Some(action) = self.actions.pop_front() {
                actor.handle(action);
            }
        }
    }

    #[derive(Default)]
    pub struct FakeSession {
        reads: VecDeque<Vec<u8>>,
        exit_status: Option<ExitStatus>,
    }

    impl FakeSession {
        pub fn with_reads(reads: Vec<Vec<u8>>) -> Self {
            Self {
                reads: reads.into(),
                exit_status: None,
            }
        }

        pub fn with_exit(mut self, status: ExitStatus) -> Self {
            self.exit_status = Some(status);
            self
        }
    }

    impl pty::Session for FakeSession {
        fn read(
            &mut self,
            buf: &mut [u8],
        ) -> std::result::Result<usize, pty::SessionError> {
            if let Some(mut chunk) = self.reads.pop_front() {
                let len = chunk.len().min(buf.len());
                buf[..len].copy_from_slice(&chunk[..len]);
                if len < chunk.len() {
                    chunk.drain(0..len);
                    self.reads.push_front(chunk);
                }
                return Ok(len);
            }
            Err(io::Error::from(io::ErrorKind::WouldBlock).into())
        }

        fn write(
            &mut self,
            input: &[u8],
        ) -> std::result::Result<usize, pty::SessionError> {
            Ok(input.len())
        }

        fn resize(
            &mut self,
            _size: pty::PtySize,
        ) -> std::result::Result<(), pty::SessionError> {
            Ok(())
        }

        fn close(&mut self) -> std::result::Result<i32, pty::SessionError> {
            Ok(0)
        }

        fn try_get_child_exit_status(
            &mut self,
        ) -> std::result::Result<Option<ExitStatus>, pty::SessionError>
        {
            Ok(self.exit_status)
        }
    }

    #[cfg(unix)]
    pub struct EioSession {
        exit_status: Option<ExitStatus>,
    }

    #[cfg(unix)]
    impl EioSession {
        pub fn with_exit(status: ExitStatus) -> Self {
            Self {
                exit_status: Some(status),
            }
        }
    }

    #[cfg(unix)]
    impl pty::Session for EioSession {
        fn read(
            &mut self,
            _buf: &mut [u8],
        ) -> std::result::Result<usize, pty::SessionError> {
            Err(io::Error::from_raw_os_error(5).into())
        }

        fn write(
            &mut self,
            _input: &[u8],
        ) -> std::result::Result<usize, pty::SessionError> {
            Ok(0)
        }

        fn resize(
            &mut self,
            _size: pty::PtySize,
        ) -> std::result::Result<(), pty::SessionError> {
            Ok(())
        }

        fn close(&mut self) -> std::result::Result<i32, pty::SessionError> {
            Ok(0)
        }

        fn try_get_child_exit_status(
            &mut self,
        ) -> std::result::Result<Option<ExitStatus>, pty::SessionError>
        {
            Ok(self.exit_status)
        }
    }

    #[derive(Default)]
    pub struct PartialSession {
        max_per_call: usize,
        block_after_first: bool,
        pub blocked: bool,
        pub writes: Vec<Vec<u8>>,
    }

    impl PartialSession {
        pub fn with_behavior(
            max_per_call: usize,
            block_after_first: bool,
        ) -> Self {
            Self {
                max_per_call,
                block_after_first,
                blocked: false,
                writes: Vec::new(),
            }
        }
    }

    impl pty::Session for PartialSession {
        fn read(
            &mut self,
            _buf: &mut [u8],
        ) -> std::result::Result<usize, otty_pty::SessionError> {
            Err(io::Error::from(io::ErrorKind::WouldBlock).into())
        }

        fn write(
            &mut self,
            input: &[u8],
        ) -> std::result::Result<usize, otty_pty::SessionError> {
            if self.block_after_first && self.blocked {
                return Err(io::Error::from(io::ErrorKind::WouldBlock).into());
            }

            let len = input.len().min(self.max_per_call);
            if len > 0 {
                self.writes.push(input[..len].to_vec());
            }
            if self.block_after_first {
                self.blocked = true;
            }
            Ok(len)
        }

        fn resize(
            &mut self,
            _size: otty_pty::PtySize,
        ) -> std::result::Result<(), otty_pty::SessionError> {
            Ok(())
        }

        fn close(
            &mut self,
        ) -> std::result::Result<i32, otty_pty::SessionError> {
            Ok(0)
        }

        fn try_get_child_exit_status(
            &mut self,
        ) -> std::result::Result<Option<ExitStatus>, otty_pty::SessionError>
        {
            Ok(None)
        }
    }

    pub fn collect_events(events: &TerminalEvents) -> Vec<TerminalEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = events.try_recv() {
            out.push(ev);
        }
        out
    }

    pub fn assert_frame(frame: SnapshotArc) {
        let view = frame.view();
        assert!(view.visible_cell_count > 0);
    }

    pub fn exit_ok() -> ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatusExt::from_raw(0)
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatusExt::from_raw(0)
        }
    }
}
