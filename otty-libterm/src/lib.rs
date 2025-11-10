mod error;
mod mode;
mod options;
mod runtime;
mod terminal;

pub use error::{LibTermError, Result};
pub use mode::TerminalMode;
pub use options::TerminalOptions;
pub use runtime::{
    PollHookHandler, Runtime, RuntimeHandle, RuntimeTarget, TerminalClient,
    TerminalEvent, TerminalRequest,
};
pub use terminal::Terminal;

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
