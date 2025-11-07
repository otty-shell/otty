mod builder;
mod error;
mod event_loop;
mod mode;
mod options;
mod terminal;

pub use error::{LibTermError, Result};
pub use event_loop::{TerminalClient, TerminalEventLoop, TerminalLoopTarget};
pub use mode::TerminalMode;
pub use options::TerminalOptions;
pub use terminal::Terminal;

#[cfg(unix)]
pub use builder::UnixTerminalBuilder;

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
pub use otty_surface::Surface as DefaultTerminalSurface;
