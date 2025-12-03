pub mod bindings;
pub mod settings;

mod block_controls;
mod engine;
mod error;
mod font;
mod input;
mod term;
mod theme;
mod view;

pub use otty_libterm::surface::{BlockSnapshot, SurfaceMode};
pub use otty_libterm::{SnapshotArc, TerminalEvent};
pub use term::{Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
