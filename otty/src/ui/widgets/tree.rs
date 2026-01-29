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
    for node in nodes {
        push_node(node, 0, &mut path, &mut entries);
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
            for child in children {
                push_node(child, depth + 1, path, entries);
            }
        }
    }

    path.pop();
}
