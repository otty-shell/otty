use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use otty_ui_tree::TreeNode;

/// Path to a node in the explorer tree, built from node names.
pub type TreePath = Vec<String>;

/// File system node used by the explorer tree.
///
/// The binary reads a folder and gives its entries back as nodes; the
/// explorer sorts nothing itself, so the reader sorts them (see the [`Ord`]
/// implementation: folders first, then by name).
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
///
/// use otty_app_explorer::FileNode;
///
/// let mut nodes = vec![
///     FileNode::new(
///         String::from("main.rs"),
///         PathBuf::from("/p/main.rs"),
///         false,
///     ),
///     FileNode::new(String::from("src"), PathBuf::from("/p/src"), true),
/// ];
/// nodes.sort();
///
/// assert_eq!(nodes[0].name(), "src");
/// ```
#[derive(Debug, Clone)]
pub struct FileNode {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_folder: bool,
    pub(crate) is_expanded: bool,
    pub(crate) children: Vec<FileNode>,
}

impl FileNode {
    /// Return display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return absolute file system path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return whether node is a folder.
    pub fn is_folder(&self) -> bool {
        self.is_folder
    }

    /// Return whether folder children are currently visible.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    /// Return nested children.
    pub fn children(&self) -> &[FileNode] {
        &self.children
    }

    /// Create a file or folder node with no children.
    pub fn new(name: String, path: PathBuf, is_folder: bool) -> Self {
        Self {
            name,
            path,
            is_folder,
            is_expanded: false,
            children: Vec::new(),
        }
    }
}

impl TreeNode for FileNode {
    fn title(&self) -> &str {
        self.name()
    }

    fn children(&self) -> Option<&[Self]> {
        if self.is_folder() {
            Some(self.children())
        } else {
            None
        }
    }

    fn expanded(&self) -> bool {
        self.is_expanded()
    }

    fn is_folder(&self) -> bool {
        self.is_folder()
    }
}

impl Ord for FileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match (!self.is_folder).cmp(&(!other.is_folder)) {
            Ordering::Equal => match compare_names(self.name(), other.name()) {
                Ordering::Equal => self.path.cmp(&other.path),
                order => order,
            },
            order => order,
        }
    }
}

impl PartialOrd for FileNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for FileNode {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FileNode {}

/// Case-insensitive comparison with case-sensitive tiebreak.
fn compare_names(left: &str, right: &str) -> Ordering {
    let left_fold = left.bytes().map(|byte| byte.to_ascii_lowercase());
    let right_fold = right.bytes().map(|byte| byte.to_ascii_lowercase());
    match left_fold.cmp(right_fold) {
        Ordering::Equal => left.cmp(right),
        order => order,
    }
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
