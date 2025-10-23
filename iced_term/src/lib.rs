pub mod actions;
pub mod bindings;
pub mod settings;

mod backend;
mod font;
mod terminal;
mod theme;
mod view;

pub use alacritty_terminal::event::Event as AlacrittyEvent;
pub use alacritty_terminal::term::TermMode;
pub use terminal::{Command, Event, Terminal};
pub use theme::{ColorPalette, Theme};
pub use view::TerminalView;
