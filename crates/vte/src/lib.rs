mod actor;
mod csi;
mod enums;
mod parser;
mod transitions;
mod utf8;

pub use actor::VTActor;
pub use csi::CsiParam;
pub use parser::Parser;

pub trait VTParser {
    fn advance<A: VTActor>(&mut self, _bytes: &[u8], _actor: &mut A) {}
}
