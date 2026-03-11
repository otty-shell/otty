//! Tree data helpers and a lightweight [`iced`] tree widget.
//!
//! This crate is split into two layers:
//! - model helpers ([`TreeNode`], [`flatten_tree`]) that are UI-agnostic;
//! - view helpers ([`TreeView`], [`TreeRowContext`]) that render rows in `iced`.
//!
//! The recommended flow for interactive trees:
//! 1. store selected/hovered paths in your app state;
//! 2. feed them into [`TreeView::selected_row`] and [`TreeView::hovered_row`];
//! 3. update that state from callbacks like [`TreeView::on_press`] and
//!    [`TreeView::on_hover`].
//!
//! See `examples/tree_view.rs` for a complete runnable example.
//!
//! # Quick Example
//!
//! ```no_run
//! use iced::widget::{container, text};
//! use iced::{Element, Length};
//! use otty_ui_tree::{TreeNode, TreePath, TreeView};
//!
//! #[derive(Clone)]
//! enum Node {
//!     Folder {
//!         title: String,
//!         expanded: bool,
//!         children: Vec<Node>,
//!     },
//!     File {
//!         title: String,
//!     },
//! }
//!
//! impl TreeNode for Node {
//!     fn title(&self) -> &str {
//!         match self {
//!             Node::Folder { title, .. } => title,
//!             Node::File { title } => title,
//!         }
//!     }
//!
//!     fn children(&self) -> Option<&[Self]> {
//!         match self {
//!             Node::Folder { children, .. } => Some(children),
//!             Node::File { .. } => None,
//!         }
//!     }
//!
//!     fn expanded(&self) -> bool {
//!         matches!(self, Node::Folder { expanded: true, .. })
//!     }
//!
//!     fn is_folder(&self) -> bool {
//!         matches!(self, Node::Folder { .. })
//!     }
//! }
//!
//! #[derive(Clone)]
//! enum Message {
//!     RowPressed(TreePath),
//!     RowHovered(Option<TreePath>),
//! }
//!
//! struct State {
//!     nodes: Vec<Node>,
//!     selected: Option<TreePath>,
//!     hovered: Option<TreePath>,
//! }
//!
//! fn view(state: &State) -> Element<'_, Message> {
//!     TreeView::new(&state.nodes, |ctx| {
//!         container(text(ctx.entry.node.title()))
//!             .width(Length::Fill)
//!             .into()
//!     })
//!     .selected_row(state.selected.as_ref())
//!     .hovered_row(state.hovered.as_ref())
//!     .on_press(Message::RowPressed)
//!     .on_hover(Message::RowHovered)
//!     .view()
//! }
//! ```

mod model;
mod view;

pub use model::{FlattenedNode, TreeNode, TreePath, flatten_tree};
pub use view::{TreeRow, TreeRowContext, TreeView};
