pub(crate) mod config;
pub(crate) mod fonts;
pub(crate) mod state;
pub(crate) mod tabs;
pub(crate) mod terminal_state;
pub(crate) mod theme;

pub(crate) use fonts::TERM_FONT_JET_BRAINS_BYTES;
pub(crate) use state::{App, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
