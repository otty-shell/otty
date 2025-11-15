mod error;
mod runtime;
mod terminal;

pub use error::{LibTermError, Result};
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
