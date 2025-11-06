mod builder;
mod error;
mod mode;
mod options;
mod terminal;

pub use error::{LibTermError, Result};
pub use mode::TerminalMode;
pub use options::TerminalOptions;
pub use terminal::{PollOutcome, Terminal, TerminalClient, TerminalSurface};

#[cfg(unix)]
pub use builder::UnixTerminalBuilder;

pub use otty_escape::{
    Action, Actor, Color, NamedPrivateMode, PrivateMode, Rgb, StdColor,
};
pub use otty_pty::PtySize;
pub use otty_surface::{
    Cell, CellAttributes, CellBlink, CellUnderline, CursorSnapshot, Grid,
    GridRow, HyperlinkRef, ScrollDirection, Surface, SurfaceConfig,
    SurfacePalette,
};
