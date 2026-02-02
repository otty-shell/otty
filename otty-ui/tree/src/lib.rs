//! UI-agnostic tree utilities plus a lightweight tree view helper.

mod model;
mod view;

pub use model::{FlattenedNode, TreeNode, TreePath, flatten_tree};
pub use view::{TreeRow, TreeRowContext, TreeView};
