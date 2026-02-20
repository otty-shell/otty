use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use otty_ui_tree::TreePath;

/// File system node used by the explorer tree.
#[derive(Debug, Clone)]
pub(crate) struct FileNode {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_folder: bool,
    pub(crate) expanded: bool,
    pub(crate) children: Vec<FileNode>,
}

impl FileNode {
    pub(crate) fn new(name: String, path: PathBuf, is_folder: bool) -> Self {
        Self {
            name,
            path,
            is_folder,
            expanded: false,
            children: Vec::new(),
        }
    }
}

/// Runtime state for the sidebar file explorer.
#[derive(Debug, Default)]
pub(crate) struct ExplorerState {
    pub(crate) root: Option<PathBuf>,
    pub(crate) root_label: Option<String>,
    pub(crate) tree: Vec<FileNode>,
    pub(crate) selected: Option<TreePath>,
    pub(crate) hovered: Option<TreePath>,
    pub(crate) last_error: Option<String>,
}

impl ExplorerState {
    /// Create a fresh explorer state with no active root.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The current root label used in the explorer header.
    pub(crate) fn root_label(&self) -> Option<&str> {
        self.root_label.as_deref()
    }

    /// Replace the explorer root and rebuild the tree if it changed.
    pub(crate) fn set_root(&mut self, root: PathBuf) {
        if self.root.as_ref() == Some(&root) {
            return;
        }

        self.root = Some(root.clone());
        self.root_label = Some(root_label(&root));
        self.selected = None;
        self.hovered = None;
        self.refresh_tree();
    }

    /// Rebuild the tree from the current root directory.
    pub(crate) fn refresh_tree(&mut self) {
        let Some(root) = self.root.as_ref() else {
            self.tree.clear();
            self.last_error = None;
            return;
        };

        match read_dir_nodes(root) {
            Ok(nodes) => {
                self.tree = nodes;
                self.last_error = None;
            },
            Err(err) => {
                self.tree.clear();
                self.last_error = Some(format!("{err}"));
                log::warn!("explorer failed to read root: {err}");
            },
        }
    }

    /// Toggle a folder node and lazily load its children.
    pub(crate) fn toggle_folder(&mut self, path: &[String]) {
        let Some(node) = find_node_mut(&mut self.tree, path) else {
            return;
        };

        if !node.is_folder {
            return;
        }

        let should_expand = !node.expanded;
        node.expanded = should_expand;
        if should_expand && node.children.is_empty() {
            match read_dir_nodes(&node.path) {
                Ok(children) => {
                    node.children = children;
                    self.last_error = None;
                },
                Err(err) => {
                    self.last_error = Some(format!("{err}"));
                    let path_display = node.path.display();
                    log::warn!(
                        "explorer failed to read folder {path_display}: {err}"
                    );
                },
            }
        }
    }

    /// Return whether the node at the provided tree path is a folder.
    pub(crate) fn node_is_folder(&self, path: &[String]) -> Option<bool> {
        find_node(&self.tree, path).map(|node| node.is_folder)
    }

    /// Resolve a tree path into its filesystem path.
    pub(crate) fn node_path(&self, path: &[String]) -> Option<PathBuf> {
        find_node(&self.tree, path).map(|node| node.path.clone())
    }
}

fn root_label(path: &Path) -> String {
    let display = path.display();
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{display}"))
}

fn read_dir_nodes(path: &Path) -> std::io::Result<Vec<FileNode>> {
    let mut nodes = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                log::warn!("explorer failed to read entry: {err}");
                continue;
            },
        };

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                log::warn!("explorer failed to read entry type: {err}");
                continue;
            },
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let is_folder = file_type.is_dir();

        nodes.push(FileNode::new(name, path, is_folder));
    }

    nodes.sort_by(compare_nodes);

    Ok(nodes)
}

fn compare_nodes(left: &FileNode, right: &FileNode) -> Ordering {
    match (!left.is_folder).cmp(&(!right.is_folder)) {
        Ordering::Equal => compare_names(&left.name, &right.name),
        other => other,
    }
}

fn compare_names(left: &str, right: &str) -> Ordering {
    let left_fold = left.bytes().map(|byte| byte.to_ascii_lowercase());
    let right_fold = right.bytes().map(|byte| byte.to_ascii_lowercase());
    match left_fold.cmp(right_fold) {
        Ordering::Equal => left.cmp(right),
        other => other,
    }
}

fn find_node<'a>(
    nodes: &'a [FileNode],
    path: &[String],
) -> Option<&'a FileNode> {
    let (head, tail) = path.split_first()?;
    let node = nodes.iter().find(|node| node.name == *head)?;

    if tail.is_empty() {
        return Some(node);
    }

    if !node.is_folder {
        return None;
    }

    find_node(&node.children, tail)
}

fn find_node_mut<'a>(
    nodes: &'a mut [FileNode],
    path: &[String],
) -> Option<&'a mut FileNode> {
    let (head, tail) = path.split_first()?;
    let index = nodes.iter().position(|node| node.name == *head)?;

    if tail.is_empty() {
        return nodes.get_mut(index);
    }

    if !nodes[index].is_folder {
        return None;
    }

    let children = &mut nodes[index].children;
    find_node_mut(children, tail)
}
