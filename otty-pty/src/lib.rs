//! Core traits and re-exports for interacting with pseudo-terminal sessions.
//!
//! This crate exposes two interchangeable backends:
//! - [`unix`] launches local command line programs attached to a PTY.
//! - [`ssh`] tunnels the interaction over an SSH connection.
//!   Both implementations conform to the [`Session`] and [`Pollable`] traits,
//!   so higher-level code can multiplex I/O and lifecycle events without
//!   caring about the transport.

mod errors;
mod size;
mod ssh;
mod unix;

use std::process::ExitStatus;

use mio::Token;

pub use crate::errors::SessionError;
pub use crate::size::PtySize;
pub use ssh::{SSHAuth, SSHSession, SSHSessionBuilder, ssh};
pub use unix::{UnixSession, UnixSessionBuilder, unix};

/// Generic PTY session that can be used interchangeably across backends.
pub trait Session {
    /// Read data from the PTY master side into the supplied buffer.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SessionError>;

    /// Write data into the PTY, forwarding it to the child process.
    fn write(&mut self, input: &[u8]) -> Result<usize, SessionError>;

    /// Request a resize of the underlying pseudo terminal.
    fn resize(&mut self, size: PtySize) -> Result<(), SessionError>;

    /// Terminate the session and return the exit code if one is available.
    fn close(&mut self) -> Result<i32, SessionError>;

    /// Poll the child process for exit status updates without blocking.
    fn try_get_child_exit_status(
        &mut self,
    ) -> Result<Option<ExitStatus>, SessionError>;
}

/// Integration point with Mio-based event loops.
pub trait Pollable: Send {
    /// Register the session's file descriptors with the provided registry.
    fn register(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<(), SessionError>;

    /// Update the interest set associated with the registered descriptors.
    fn reregister(
        &mut self,
        registry: &mio::Registry,
        interest: mio::Interest,
        io_token: Token,
        child_token: Token,
    ) -> Result<(), SessionError>;

    /// Remove the session's resources from the registry.
    fn deregister(
        &mut self,
        registry: &mio::Registry,
    ) -> Result<(), SessionError>;
}
