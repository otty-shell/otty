use std::path::PathBuf;

use super::model::{FileNode, TreePath};
use super::services::root_label;

/// Runtime state for the sidebar file explorer.
#[derive(Debug, Default)]
pub(crate) struct ExplorerState {
    root: Option<PathBuf>,
    root_label: Option<String>,
    tree: Vec<FileNode>,
    selected: Option<TreePath>,
    hovered: Option<TreePath>,
}

impl ExplorerState {
    // --- Read access ---

    /// Return the current root directory path.
    pub(crate) fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }

    /// Return current root label used in the explorer header.
    pub(crate) fn root_label(&self) -> Option<&str> {
        self.root_label.as_deref()
    }

    /// Return root tree entries.
    pub(crate) fn tree(&self) -> &[FileNode] {
        &self.tree
    }

    /// Return selected tree path.
    pub(crate) fn selected_path(&self) -> Option<&TreePath> {
        self.selected.as_ref()
    }

    /// Return hovered tree path.
    pub(crate) fn hovered_path(&self) -> Option<&TreePath> {
        self.hovered.as_ref()
    }

    // --- Write access ---

    /// Update selected tree path.
    pub(super) fn set_selected_path(&mut self, path: Option<TreePath>) {
        self.selected = path;
    }

    /// Update hovered tree path.
    pub(super) fn set_hovered_path(&mut self, path: Option<TreePath>) {
        self.hovered = path;
    }

    /// Set explorer root and clear current tree if changed.
    ///
    /// Returns `true` when the root actually changed.
    pub(super) fn set_root(&mut self, root: Option<PathBuf>) -> bool {
        if self.root == root {
            return false;
        }

        self.root_label = root.as_deref().map(root_label);
        self.root = root;
        self.tree.clear();
        self.selected = None;
        self.hovered = None;
        true
    }

    /// Apply root children after async load completion.
    ///
    /// Returns `false` when the root has changed since the load was started.
    pub(super) fn apply_root_nodes(
        &mut self,
        root: &PathBuf,
        nodes: Vec<FileNode>,
    ) -> bool {
        if self.root.as_ref() != Some(root) {
            return false;
        }

        self.tree = nodes;
        true
    }

    /// Toggle folder state and return path to lazily load, if required.
    pub(super) fn toggle_folder(&mut self, path: &[String]) -> Option<PathBuf> {
        let node = find_node_mut(&mut self.tree, path)?;
        if !node.is_folder() {
            return None;
        }

        let should_expand = !node.is_expanded();
        node.is_expanded = should_expand;
        if should_expand && node.children().is_empty() {
            return Some(node.path().to_path_buf());
        }

        None
    }

    /// Apply folder children after async load completion.
    pub(super) fn apply_folder_nodes(
        &mut self,
        path: &[String],
        children: Vec<FileNode>,
    ) -> bool {
        let Some(node) = find_node_mut(&mut self.tree, path) else {
            return false;
        };
        if !node.is_folder() || !node.is_expanded() {
            return false;
        }

        node.children = children;
        true
    }

    /// Return whether the node at the provided tree path is a folder.
    pub(super) fn node_is_folder(&self, path: &[String]) -> Option<bool> {
        find_node(&self.tree, path).map(FileNode::is_folder)
    }

    /// Resolve a tree path into its filesystem path.
    pub(super) fn node_path(&self, path: &[String]) -> Option<PathBuf> {
        find_node(&self.tree, path).map(|node| node.path().to_path_buf())
    }
}

fn find_node<'a>(
    nodes: &'a [FileNode],
    path: &[String],
) -> Option<&'a FileNode> {
    let (head, tail) = path.split_first()?;
    let index = find_child_index(nodes, head)?;
    let node = nodes.get(index)?;

    if tail.is_empty() {
        return Some(node);
    }

    if !node.is_folder() {
        return None;
    }

    find_node(node.children(), tail)
}

fn find_node_mut<'a>(
    nodes: &'a mut [FileNode],
    path: &[String],
) -> Option<&'a mut FileNode> {
    let (head, tail) = path.split_first()?;
    let index = find_child_index(nodes, head)?;
    let node = nodes.get_mut(index)?;

    if tail.is_empty() {
        return Some(node);
    }

    if !node.is_folder() {
        return None;
    }

    find_node_mut(&mut node.children, tail)
}

fn find_child_index(nodes: &[FileNode], name: &str) -> Option<usize> {
    nodes.iter().position(|node| node.name() == name)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ExplorerState;
    use crate::widgets::explorer::model::FileNode;

    #[test]
    fn given_new_root_when_set_root_then_tree_is_reset() {
        let mut state = ExplorerState::default();

        assert!(state.set_root(Some(PathBuf::from("/tmp"))));
        assert_eq!(state.root_label(), Some("tmp"));
        assert!(state.tree().is_empty());
    }

    #[test]
    fn given_same_root_when_set_root_then_returns_false() {
        let mut state = ExplorerState::default();

        assert!(state.set_root(Some(PathBuf::from("/tmp"))));
        assert!(!state.set_root(Some(PathBuf::from("/tmp"))));
    }

    #[test]
    fn given_expand_folder_with_empty_children_when_toggled_then_load_path_returned()
     {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let load_path = state.toggle_folder(&[String::from("src")]);

        assert_eq!(load_path, Some(root.join("src")));
    }

    #[test]
    fn given_loaded_folder_children_when_applied_then_node_children_updated() {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        let _ = state.toggle_folder(&[String::from("src")]);

        let changed = state.apply_folder_nodes(
            &[String::from("src")],
            vec![FileNode::new(
                String::from("main.rs"),
                root.join("src/main.rs"),
                false,
            )],
        );

        assert!(changed);
        let path =
            state.node_path(&[String::from("src"), String::from("main.rs")]);
        assert_eq!(path, Some(root.join("src/main.rs")));
    }
}
