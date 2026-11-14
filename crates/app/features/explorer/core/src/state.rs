use std::path::{Path, PathBuf};

use iced::Point;

use crate::interaction::{
    ContextMenu, Drag, DropTarget, InlineEdit, PendingDelete,
};
use crate::types::{FileNode, TreePath};

/// Runtime state for the sidebar file explorer.
#[derive(Debug, Default)]
pub(crate) struct ExplorerState {
    root: Option<PathBuf>,
    root_label: Option<String>,
    tree: Vec<FileNode>,
    selected: Option<TreePath>,
    hovered: Option<TreePath>,
    pressed: Option<TreePath>,
    /// Last pointer position over the tree, in the tree's own coordinates.
    cursor: Point,
    context_menu: Option<ContextMenu>,
    inline_edit: Option<InlineEdit>,
    pending_delete: Option<PendingDelete>,
    drag: Option<Drag>,
    drop_target: Option<DropTarget>,
}

impl ExplorerState {
    /// Return the followed folder.
    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
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

    /// Return the row held down by the pointer.
    pub(crate) fn pressed_path(&self) -> Option<&TreePath> {
        self.pressed.as_ref()
    }

    /// Return the last pointer position over the tree.
    pub(crate) fn cursor(&self) -> Point {
        self.cursor
    }

    /// Return the open context menu.
    pub(crate) fn context_menu(&self) -> Option<&ContextMenu> {
        self.context_menu.as_ref()
    }

    /// Return the name being typed in the tree.
    pub(crate) fn inline_edit(&self) -> Option<&InlineEdit> {
        self.inline_edit.as_ref()
    }

    /// Return the deletion waiting for its answer.
    pub(crate) fn pending_delete(&self) -> Option<&PendingDelete> {
        self.pending_delete.as_ref()
    }

    /// Return the row held down or dragged.
    pub(crate) fn drag(&self) -> Option<&Drag> {
        self.drag.as_ref()
    }

    /// Return where a dragged row would land now.
    pub(crate) fn drop_target(&self) -> Option<&DropTarget> {
        self.drop_target.as_ref()
    }

    /// Return whether a menu, a dialog or a name field of Files is open.
    pub(crate) fn has_overlay(&self) -> bool {
        self.context_menu.is_some()
            || self.inline_edit.is_some()
            || self.pending_delete.is_some()
    }

    /// Update selected tree path.
    pub(crate) fn set_selected_path(&mut self, path: Option<TreePath>) {
        self.selected = path;
    }

    /// Update hovered tree path.
    pub(crate) fn set_hovered_path(&mut self, path: Option<TreePath>) {
        self.hovered = path;
    }

    /// Update the row held down by the pointer.
    pub(crate) fn set_pressed_path(&mut self, path: Option<TreePath>) {
        self.pressed = path;
    }

    /// Update the last pointer position over the tree.
    pub(crate) fn set_cursor(&mut self, position: Point) {
        self.cursor = position;
    }

    /// Open a context menu, or close it with `None`.
    pub(crate) fn set_context_menu(&mut self, menu: Option<ContextMenu>) {
        self.context_menu = menu;
    }

    /// Close the context menu and return it.
    pub(crate) fn take_context_menu(&mut self) -> Option<ContextMenu> {
        self.context_menu.take()
    }

    /// Start typing a name, or stop with `None`.
    pub(crate) fn set_inline_edit(&mut self, edit: Option<InlineEdit>) {
        self.inline_edit = edit;
    }

    /// Return the name being typed, to change it.
    pub(crate) fn inline_edit_mut(&mut self) -> Option<&mut InlineEdit> {
        self.inline_edit.as_mut()
    }

    /// Stop typing a name and return it.
    pub(crate) fn take_inline_edit(&mut self) -> Option<InlineEdit> {
        self.inline_edit.take()
    }

    /// Ask the user before a deletion, or forget the question with `None`.
    pub(crate) fn set_pending_delete(&mut self, delete: Option<PendingDelete>) {
        self.pending_delete = delete;
    }

    /// Forget the question about a deletion and return it.
    pub(crate) fn take_pending_delete(&mut self) -> Option<PendingDelete> {
        self.pending_delete.take()
    }

    /// Hold a row down, or let it go with `None`.
    pub(crate) fn set_drag(&mut self, drag: Option<Drag>) {
        self.drag = drag;
    }

    /// Return the held row, to change it.
    pub(crate) fn drag_mut(&mut self) -> Option<&mut Drag> {
        self.drag.as_mut()
    }

    /// Let the held row go and return it.
    pub(crate) fn take_drag(&mut self) -> Option<Drag> {
        self.drag.take()
    }

    /// Update where a dragged row would land.
    pub(crate) fn set_drop_target(&mut self, target: Option<DropTarget>) {
        self.drop_target = target;
    }

    /// Set explorer root and clear current tree if changed. The menu, the
    /// name field and a drag of the old tree go with it; a deletion that
    /// waits for its answer names its entry by its full path, so it stays.
    ///
    /// Returns `true` when the root actually changed.
    pub(crate) fn set_root(&mut self, root: Option<PathBuf>) -> bool {
        if self.root == root {
            return false;
        }

        self.root_label = root.as_deref().map(root_label);
        self.root = root;
        self.tree.clear();
        self.selected = None;
        self.hovered = None;
        self.pressed = None;
        self.context_menu = None;
        self.inline_edit = None;
        self.drag = None;
        self.drop_target = None;
        true
    }

    /// Apply root children after async load completion.
    ///
    /// Returns `false` when the root has changed since the load was started.
    pub(crate) fn apply_root_nodes(
        &mut self,
        root: &PathBuf,
        nodes: Vec<FileNode>,
    ) -> bool {
        if self.root.as_ref() != Some(root) {
            return false;
        }

        self.tree = merge_nodes(std::mem::take(&mut self.tree), nodes);
        true
    }

    /// Toggle folder state and return path to lazily load, if required.
    pub(crate) fn toggle_folder(&mut self, path: &[String]) -> Option<PathBuf> {
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
    pub(crate) fn apply_folder_nodes(
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

    /// Apply direct children for the directory at a filesystem path.
    ///
    /// Returns `false` when the directory does not belong to the currently
    /// loaded explorer tree.
    pub(crate) fn apply_directory_nodes(
        &mut self,
        directory: &Path,
        nodes: Vec<FileNode>,
    ) -> bool {
        if self.root.as_deref() == Some(directory) {
            self.tree = merge_nodes(std::mem::take(&mut self.tree), nodes);
            return true;
        }

        let Some(node) = find_folder_mut_by_path(&mut self.tree, directory)
        else {
            return false;
        };
        if !is_loaded_directory(node) {
            return false;
        }

        node.children = merge_nodes(std::mem::take(&mut node.children), nodes);
        true
    }

    /// Open the folder at `path` without closing an open one, and return
    /// its path on disk when it still has to be read.
    pub(crate) fn expand_folder(&mut self, path: &[String]) -> Option<PathBuf> {
        let node = find_node_mut(&mut self.tree, path)?;
        if !node.is_folder() || node.is_expanded() {
            return None;
        }

        node.is_expanded = true;
        if node.children().is_empty() {
            return Some(node.path().to_path_buf());
        }

        None
    }

    /// Return the node at the provided tree path.
    pub(crate) fn node(&self, path: &[String]) -> Option<&FileNode> {
        find_node(&self.tree, path)
    }

    /// Return the folder on disk at `path`: the followed folder for an empty
    /// path, `None` when the path is no folder of the tree.
    pub(crate) fn folder_on_disk(&self, path: &[String]) -> Option<PathBuf> {
        if path.is_empty() {
            return self.root.clone();
        }

        find_node(&self.tree, path)
            .filter(|node| node.is_folder())
            .map(|node| node.path().to_path_buf())
    }

    /// Return the names of the entries the tree shows in the folder at
    /// `path`; empty for a folder that was not read yet.
    pub(crate) fn names_in_folder(&self, path: &[String]) -> Vec<&str> {
        let children = if path.is_empty() {
            self.tree.as_slice()
        } else {
            match find_node(&self.tree, path) {
                Some(node) if node.is_folder() => node.children(),
                _ => &[],
            }
        };

        children.iter().map(FileNode::name).collect()
    }

    /// Return whether the node at the provided tree path is a folder.
    pub(crate) fn node_is_folder(&self, path: &[String]) -> Option<bool> {
        find_node(&self.tree, path).map(FileNode::is_folder)
    }

    /// Resolve a tree path into its filesystem path.
    pub(crate) fn node_path(&self, path: &[String]) -> Option<PathBuf> {
        find_node(&self.tree, path).map(|node| node.path().to_path_buf())
    }

    /// Return directories whose direct children should be watched.
    pub(crate) fn watched_directories(&self) -> Vec<PathBuf> {
        let Some(root) = self.root.as_ref() else {
            return Vec::new();
        };

        let mut directories = vec![root.clone()];
        collect_loaded_directories(&self.tree, &mut directories);
        directories.sort();
        directories.dedup();
        directories
    }

    /// Return whether the path is a currently watched explorer directory.
    pub(crate) fn is_watched_directory(&self, directory: &Path) -> bool {
        if self.root.as_deref() == Some(directory) {
            return true;
        }

        find_folder_by_path(&self.tree, directory)
            .map(is_loaded_directory)
            .unwrap_or(false)
    }
}

/// Build explorer root label from a file system path.
///
/// Returns the final path component, or the full display string for root paths.
fn root_label(path: &Path) -> String {
    let display = path.display();
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{display}"))
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

fn find_folder_by_path<'a>(
    nodes: &'a [FileNode],
    path: &Path,
) -> Option<&'a FileNode> {
    for node in nodes {
        if node.is_folder() && node.path() == path {
            return Some(node);
        }

        if node.is_folder()
            && let Some(found) = find_folder_by_path(node.children(), path)
        {
            return Some(found);
        }
    }

    None
}

fn find_folder_mut_by_path<'a>(
    nodes: &'a mut [FileNode],
    path: &Path,
) -> Option<&'a mut FileNode> {
    for node in nodes {
        if node.is_folder() && node.path() == path {
            return Some(node);
        }

        if node.is_folder()
            && let Some(found) =
                find_folder_mut_by_path(&mut node.children, path)
        {
            return Some(found);
        }
    }

    None
}

fn find_child_index(nodes: &[FileNode], name: &str) -> Option<usize> {
    nodes.iter().position(|node| node.name() == name)
}

fn collect_loaded_directories(
    nodes: &[FileNode],
    directories: &mut Vec<PathBuf>,
) {
    for node in nodes {
        if !node.is_folder() {
            continue;
        }

        if is_loaded_directory(node) {
            directories.push(node.path().to_path_buf());
        }

        collect_loaded_directories(node.children(), directories);
    }
}

fn is_loaded_directory(node: &FileNode) -> bool {
    node.is_folder() && (node.is_expanded() || !node.children().is_empty())
}

fn merge_nodes(
    previous: Vec<FileNode>,
    mut next: Vec<FileNode>,
) -> Vec<FileNode> {
    let mut previous = previous;
    for node in &mut next {
        let Some(index) = previous.iter().position(|candidate| {
            candidate.path == node.path && candidate.is_folder == node.is_folder
        }) else {
            continue;
        };

        preserve_folder_state(node, previous.swap_remove(index));
    }

    next
}

fn preserve_folder_state(node: &mut FileNode, previous: FileNode) {
    if !node.is_folder || !previous.is_folder {
        return;
    }

    node.is_expanded = previous.is_expanded;
    if node.is_expanded || !previous.children.is_empty() {
        node.children = previous.children;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ExplorerState, root_label};
    use crate::types::FileNode;

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

    #[test]
    fn given_root_reloaded_when_folder_was_expanded_then_expansion_is_preserved()
     {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        let _ = state.toggle_folder(&[String::from("src")]);
        let _ = state.apply_folder_nodes(
            &[String::from("src")],
            vec![FileNode::new(
                String::from("main.rs"),
                root.join("src/main.rs"),
                false,
            )],
        );

        let changed = state.apply_directory_nodes(
            &root,
            vec![
                FileNode::new(String::from("src"), root.join("src"), true),
                FileNode::new(
                    String::from("Cargo.toml"),
                    root.join("Cargo.toml"),
                    false,
                ),
            ],
        );

        assert!(changed);
        assert_eq!(state.tree().len(), 2);
        assert!(state.tree()[0].is_expanded());
        assert_eq!(state.tree()[0].children().len(), 1);
        assert_eq!(state.tree()[0].children()[0].name(), "main.rs");
    }

    #[test]
    fn given_loaded_folder_reloaded_when_child_folder_was_expanded_then_nested_expansion_is_preserved()
     {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        let _ = state.toggle_folder(&[String::from("src")]);
        let _ = state.apply_folder_nodes(
            &[String::from("src")],
            vec![FileNode::new(
                String::from("nested"),
                root.join("src/nested"),
                true,
            )],
        );
        let _ =
            state.toggle_folder(&[String::from("src"), String::from("nested")]);
        let _ = state.apply_folder_nodes(
            &[String::from("src"), String::from("nested")],
            vec![FileNode::new(
                String::from("lib.rs"),
                root.join("src/nested/lib.rs"),
                false,
            )],
        );

        let changed = state.apply_directory_nodes(
            &root.join("src"),
            vec![
                FileNode::new(
                    String::from("nested"),
                    root.join("src/nested"),
                    true,
                ),
                FileNode::new(
                    String::from("main.rs"),
                    root.join("src/main.rs"),
                    false,
                ),
            ],
        );

        assert!(changed);
        let src = &state.tree()[0];
        assert_eq!(src.children().len(), 2);
        assert!(src.children()[0].is_expanded());
        assert_eq!(src.children()[0].children().len(), 1);
        assert_eq!(src.children()[0].children()[0].name(), "lib.rs");
    }

    #[test]
    fn given_loaded_folder_when_watch_directories_requested_then_root_and_loaded_folder_are_returned()
     {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        let _ = state.toggle_folder(&[String::from("src")]);
        let _ = state.apply_folder_nodes(
            &[String::from("src")],
            vec![FileNode::new(
                String::from("main.rs"),
                root.join("src/main.rs"),
                false,
            )],
        );
        let _ = state.toggle_folder(&[String::from("src")]);

        let directories = state.watched_directories();

        assert_eq!(directories, vec![root.clone(), root.join("src")]);
    }

    #[test]
    fn given_root_path_when_label_requested_then_uses_display() {
        assert_eq!(root_label(std::path::Path::new("/")), "/");
    }

    #[test]
    fn given_normal_path_when_label_requested_then_returns_file_name() {
        assert_eq!(
            root_label(std::path::Path::new("/home/user/projects")),
            "projects"
        );
    }
}
