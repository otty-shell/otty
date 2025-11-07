mod actor;
mod attributes;
mod charset;
mod color;
mod control;
mod csi;
mod cursor;
mod esc;
mod hyperlink;
mod keyboard;
mod mode;
mod osc;
mod parser;

pub use actor::{Action, EscapeActor};
pub use attributes::CharacterAttribute;
pub use charset::{Charset, CharsetIndex};
pub use color::{Color, Rgb, StdColor};
pub use cursor::{CursorShape, CursorStyle};
pub use hyperlink::Hyperlink;
pub use keyboard::*;
pub use mode::*;
pub use otty_vte as vte;
pub use parser::Parser;

pub trait EscapeParser {
    fn advance<A: EscapeActor>(&mut self, _bytes: &[u8], _actor: &mut A) {}
}
