//! Terminal runtime glue for PTY sessions, escape parsing and surface state.
//!
//! This crate connects the lower-level building blocks from the OTTY
//! workspace:
//! - [`otty_pty`] for spawning and driving PTY or SSH sessions,
//! - [`otty_escape`] for parsing terminal escape sequences into semantic
//!   actions,
//! - [`otty_surface`] for maintaining an in-memory terminal screen model.
//!
//! The main entry points are:
//! - [`Terminal`], which owns a PTY session, escape parser and surface, and
//!   exposes a high-level API (`TerminalRequest` / `TerminalEvent`),
//! - [`Runtime`], a small `mio`-based event loop that drives a
//!   [`RuntimeClient`] (typically a [`Terminal`]) based on OS events and
//!   queued requests.
//!
//! Front-ends usually:
//! 1. Construct a PTY [`pty::Session`], an [`escape::EscapeParser`] instance
//!    and an [`surface::SurfaceActor`] implementation.
//! 2. Wrap them in a [`Terminal`].
//! 3. Create a [`Runtime`], obtain a [`RuntimeRequestProxy`] from it and keep
//!    it on the UI side.
//! 4. Run the `Runtime` loop with the `Terminal` as its client, while the UI
//!    sends `TerminalRequest`s through the proxy and reacts to
//!    [`TerminalEvent`]s emitted by the terminal.

mod error;
mod runtime;
mod terminal;

pub use error::{Error, Result};
pub use runtime::{
    Runtime, RuntimeClient, RuntimeEvent, RuntimeHooks, RuntimeRequestProxy,
};
pub use terminal::{
    Terminal, TerminalClient, TerminalEvent, TerminalRequest,
    options::TerminalOptions, size::TerminalSize, snapshot::TerminalSnapshot,
};

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
