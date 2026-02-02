//! Tree utilities shared by UI widgets.
//!
//! This module defines a small UI-facing tree model:
//! - [`TreeNode`] describes a node with a title, optional children,
//!   and expansion state.
//! - [`flatten_tree`] turns a nested tree into a flat list of visible rows
//!   with depth and path metadata, suitable for rendering.
//! - [`TreePath`] is the path of titles from the root to a node, used for
//!   selection/hover/expand events in widgets.
//!
//! It is intentionally UI-agnostic (no rendering code). Widgets like
//! `quick_commands` and `settings` provide their own row rendering while
//! reusing the same tree flattening logic.
//!
//! # Usage
//!
//! Single example that defines folders/files, flattens the tree, and renders
//! it with iced:
//!
//! ```rust,ignore
//! use iced::widget::{column, row, text, Space};
//! use iced::{Element, Length};
//! use crate::ui::widgets::tree::{TreeNode, flatten_tree};
//!
//! #[derive(Clone)]
//! enum Node {
//!     Folder { title: String, expanded: bool, children: Vec<Node> },
//!     File { title: String },
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
//!         match self {
//!             Node::Folder { expanded, .. } => *expanded,
//!             Node::File { .. } => false,
//!         }
//!     }
//!
//!     fn is_folder(&self) -> bool {
//!         matches!(self, Node::Folder { .. })
//!     }
//! }
//!
//! fn render_tree<'a>(nodes: &'a [Node]) -> Element<'a, Event> {
//!     let mut col = column![];
//!     for entry in flatten_tree(nodes) {
//!         let indent = entry.depth as f32 * 14.0;
//!         let row = row![
//!             Space::new().width(Length::Fixed(indent)),
//!             text(entry.node.title()),
//!         ]
//!         .spacing(6);
//!         col = col.push(row);
//!     }
//!     col.into()
//! }
//!
//! let tree = vec![
//!     Node::Folder {
//!         title: "General".to_string(),
//!         expanded: true,
//!         children: vec![
//!             Node::File { title: "Terminal".to_string() },
//!             Node::File { title: "Theme".to_string() },
//!         ],
//!     },
//!     Node::File { title: "About".to_string() },
//! ];
//!
//! let _view = render_tree(&tree);
//! ```
//!
//! Typical widget flow:
//! 1. Keep your own tree model in state.
//! 2. Call [`flatten_tree`] in `view`.
//! 3. Render each row using `entry.depth` for indentation.
//! 4. Use `entry.path` to drive selection, hover, and expand/collapse.
//!
/// Path of titles that identifies a tree node from the root.
pub(crate) type TreePath = Vec<String>;

/// Trait implemented by tree node types that expose children and expansion.
pub(crate) trait TreeNode {
    /// Title used to identify the node within its parent.
    fn title(&self) -> &str;
    /// Children for the node (folders only).
    fn children(&self) -> Option<&[Self]>
    where
        Self: Sized;
    /// Whether the node is expanded (folders only).
    fn expanded(&self) -> bool;
    /// Whether this node is a folder.
    fn is_folder(&self) -> bool;
}

/// Flattened representation of a tree node with its depth and path.
pub(crate) struct FlattenedNode<'a, T: TreeNode> {
    pub(crate) depth: usize,
    pub(crate) node: &'a T,
    pub(crate) path: TreePath,
}

/// Flatten a tree of nodes into a depth-first list of visible entries.
pub(crate) fn flatten_tree<'a, T: TreeNode>(
    nodes: &'a [T],
) -> Vec<FlattenedNode<'a, T>> {
    let mut entries = Vec::new();
    let mut path = Vec::new();
    for index in sorted_indices(nodes) {
        push_node(&nodes[index], 0, &mut path, &mut entries);
    }
    entries
}

fn push_node<'a, T: TreeNode>(
    node: &'a T,
    depth: usize,
    path: &mut Vec<String>,
    entries: &mut Vec<FlattenedNode<'a, T>>,
) {
    path.push(node.title().to_string());
    entries.push(FlattenedNode {
        depth,
        node,
        path: path.clone(),
    });

    if node.is_folder() && node.expanded() {
        if let Some(children) = node.children() {
            for index in sorted_indices(children) {
                push_node(&children[index], depth + 1, path, entries);
            }
        }
    }

    path.pop();
}

fn sorted_indices<T: TreeNode>(nodes: &[T]) -> Vec<usize> {
    let mut ordered: Vec<usize> = (0..nodes.len()).collect();
    ordered.sort_by(|a, b| {
        let left = &nodes[*a];
        let right = &nodes[*b];
        match (!left.is_folder()).cmp(&(!right.is_folder())) {
            std::cmp::Ordering::Equal => {
                compare_titles(left.title(), right.title())
            },
            other => other,
        }
    });
    ordered
}

fn compare_titles(left: &str, right: &str) -> std::cmp::Ordering {
    let left_segments = split_segments(left);
    let right_segments = split_segments(right);
    let mut left_iter = left_segments.iter();
    let mut right_iter = right_segments.iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(Segment::Digits(l)), Some(Segment::Digits(r))) => {
                let ord = compare_digit_segments(l, r);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            },
            (Some(Segment::Text(l)), Some(Segment::Text(r))) => {
                let ord = compare_text_segments(l, r);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            },
            (Some(Segment::Digits(_)), Some(Segment::Text(_))) => {
                return std::cmp::Ordering::Less;
            },
            (Some(Segment::Text(_)), Some(Segment::Digits(_))) => {
                return std::cmp::Ordering::Greater;
            },
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => break,
        }
    }

    left.cmp(right)
}

#[derive(Debug)]
enum Segment {
    Text(String),
    Digits(String),
}

fn split_segments(input: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for ch in input.chars() {
        let is_digit = ch.is_ascii_digit();
        match current_is_digit {
            Some(kind) if kind == is_digit => current.push(ch),
            Some(kind) => {
                segments.push(if kind {
                    Segment::Digits(current)
                } else {
                    Segment::Text(current)
                });
                current = String::new();
                current.push(ch);
                current_is_digit = Some(is_digit);
            },
            None => {
                current.push(ch);
                current_is_digit = Some(is_digit);
            },
        }
    }

    if let Some(kind) = current_is_digit {
        segments.push(if kind {
            Segment::Digits(current)
        } else {
            Segment::Text(current)
        });
    }

    segments
}

fn compare_text_segments(left: &str, right: &str) -> std::cmp::Ordering {
    let left_lower = left.to_lowercase();
    let right_lower = right.to_lowercase();
    match left_lower.cmp(&right_lower) {
        std::cmp::Ordering::Equal => left.cmp(right),
        other => other,
    }
}

fn compare_digit_segments(left: &str, right: &str) -> std::cmp::Ordering {
    let left_trim = left.trim_start_matches('0');
    let right_trim = right.trim_start_matches('0');

    let left_trim = if left_trim.is_empty() { "0" } else { left_trim };
    let right_trim = if right_trim.is_empty() {
        "0"
    } else {
        right_trim
    };

    match left_trim.len().cmp(&right_trim.len()) {
        std::cmp::Ordering::Equal => match left_trim.cmp(right_trim) {
            std::cmp::Ordering::Equal => left.len().cmp(&right.len()),
            other => other,
        },
        other => other,
    }
}
