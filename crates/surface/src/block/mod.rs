//! Command blocks: the block surface that keeps a terminal surface for each
//! block and stitches them into one history, what a snapshot says about each
//! block, the folding of a block and the text of a block for copying.

mod fold;
mod snapshot;
mod surface;
#[cfg(test)]
pub(crate) mod testing;
mod text;

pub use self::snapshot::{BlockKey, BlockKind, BlockMeta, BlockSnapshot};
pub use self::surface::BlockSurface;
