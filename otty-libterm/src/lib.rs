mod error;
mod mode;
mod runtime;
mod snapshot;
mod terminal;

pub use error::{LibTermError, Result};
pub use mode::TerminalMode;
pub use runtime::{
    Runtime, RuntimeClient, RuntimeEvent, RuntimeHooks, RuntimeRequestProxy,
};
pub use snapshot::TerminalSnapshot;
pub use terminal::{
    Terminal, TerminalClient, TerminalEvent, TerminalOptions, TerminalRequest,
};

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
