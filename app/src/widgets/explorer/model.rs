use otty_ui_tree::TreePath;

use super::types::FileNode;

/// Read-only view model for the explorer tree.
#[derive(Debug, Clone)]
pub(crate) struct ExplorerTreeViewModel<'a> {
    pub(super) root_label: Option<&'a str>,
    pub(super) tree: &'a [FileNode],
    pub(super) selected_path: Option<&'a TreePath>,
    pub(super) hovered_path: Option<&'a TreePath>,
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::path::PathBuf;

    use super::FileNode;

    #[test]
    fn given_mixed_nodes_when_sorted_then_folders_are_first() {
        let mut nodes = [
            FileNode::new(
                String::from("b.rs"),
                PathBuf::from("/tmp/b.rs"),
                false,
            ),
            FileNode::new(String::from("src"), PathBuf::from("/tmp/src"), true),
            FileNode::new(
                String::from("a.rs"),
                PathBuf::from("/tmp/a.rs"),
                false,
            ),
        ];

        nodes.sort();

        assert!(nodes[0].is_folder());
        assert_eq!(nodes[1].name(), "a.rs");
        assert_eq!(nodes[2].name(), "b.rs");
    }

    #[test]
    fn given_same_name_with_case_difference_when_compared_then_order_is_stable()
    {
        let lower = FileNode::new(
            String::from("readme"),
            PathBuf::from("/tmp/readme"),
            false,
        );
        let upper = FileNode::new(
            String::from("README"),
            PathBuf::from("/tmp/README"),
            false,
        );

        assert_ne!(lower.cmp(&upper), Ordering::Equal);
    }
}
