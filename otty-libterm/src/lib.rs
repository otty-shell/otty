mod error;
mod mode;
mod runtime;
mod snapshot;
mod terminal;

pub use error::{LibTermError, Result};
pub use mode::TerminalMode;
pub use runtime::{
    PollHookHandler, Runtime, RuntimeClient, RuntimeEvent, RuntimeHandle,
};
pub use snapshot::{SurfaceSnapshot, SurfaceSnapshotSource, TerminalSnapshot};
pub use terminal::{Terminal, TerminalEvent, TerminalClient, TerminalOptions, TerminalRequest};

pub use otty_escape as escape;
pub use otty_pty as pty;
pub use otty_surface as surface;
