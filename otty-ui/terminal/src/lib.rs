pub mod bindings;
pub mod settings;

mod block_controls;
mod block_layout;
mod engine;
mod error;
mod font;
mod input;
mod term;
mod theme;
mod view;

pub use block_controls::{
    BlockActionButtonGeometry, compute_action_button_geometry,
};
pub use block_layout::{BlockRect, block_rects};
pub use font::font_measure;
pub use otty_libterm::surface::{BlockKind, BlockSnapshot, SurfaceMode};
pub use otty_libterm::{SnapshotArc, TerminalEvent};
pub use term::{BlockCommand, BlockUiMode, Event, Terminal};
pub use theme::{ColorPalette, Theme, parse_hex_color};
pub use view::TerminalView;
