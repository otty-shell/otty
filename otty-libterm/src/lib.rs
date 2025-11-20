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
//!    readiness model, and drain [`TerminalEvent`]s with `next_event()`.

mod error;
mod runtime;
mod terminal;

pub use error::{Error, Result};
pub use runtime::{
    Runtime, RuntimeClient, RuntimeEvent, RuntimeHooks, RuntimeRequestProxy,
};
pub use terminal::{
    TerminalEngine, TerminalEvent, TerminalRequest, options::TerminalOptions,
    size::TerminalSize,
};

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
