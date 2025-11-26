pub mod actions;
pub mod bindings;
pub mod settings;

mod error;
mod engine;
mod font;
mod term;
mod theme;
mod view;

pub use otty_libterm::surface::SurfaceMode;
pub use otty_libterm::TerminalEvent;
pub use term::{Request, Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
