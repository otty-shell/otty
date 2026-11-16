pub mod bindings;
pub mod settings;

mod block_layout;
mod block_paint;
mod block_scroll;
mod clipboard;
mod engine;
mod error;
mod font;
mod input;
mod paste;
mod program;
mod row_layout;
mod scroll_mark;
mod term;
#[cfg(test)]
mod testing;
mod text;
mod theme;
mod view;

pub use block_layout::BlockRect;
pub use error::Error;
pub use font::font_measure;
pub use otty_libterm::surface::{
    BlockKey, BlockKind, BlockSnapshot, SnapshotOwned, SnapshotView,
    SurfaceMode,
};
pub use otty_libterm::{TerminalEvent, TerminalRequest};
pub use program::{ProgramEventSender, StartedProgram, TerminalProgram};
pub use term::{BlockCommand, Event, Terminal};
pub use theme::{ColorPalette, Theme, parse_hex_color};
pub use view::{TEXT_INSET_X, TerminalView};
