pub mod bindings;
pub mod settings;

mod engine;
mod error;
mod font;
mod term;
mod theme;
mod view;

pub use otty_libterm::TerminalEvent;
pub use otty_libterm::surface::SurfaceMode;
pub use term::{Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
