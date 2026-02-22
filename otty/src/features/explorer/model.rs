use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// File system node used by the explorer tree.
#[derive(Debug, Clone)]
pub(crate) struct FileNode {
    name: String,
    path: PathBuf,
    is_folder: bool,
    expanded: bool,
    children: Vec<FileNode>,
}

impl FileNode {
    /// Create a file or folder node.
    pub(crate) fn new(name: String, path: PathBuf, is_folder: bool) -> Self {
        Self {
            name,
            path,
            is_folder,
            expanded: false,
            children: Vec::new(),
        }
    }

    /// Return display name.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Return absolute file system path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Return whether node is a folder.
    pub(crate) fn is_folder(&self) -> bool {
        self.is_folder
    }

    /// Return whether folder children are currently expanded.
    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Return nested children.
    pub(crate) fn children(&self) -> &[FileNode] {
        &self.children
    }

    /// Return mutable nested children.
    pub(crate) fn children_mut(&mut self) -> &mut Vec<FileNode> {
        &mut self.children
    }

    /// Replace children with freshly loaded values.
    pub(crate) fn set_children(&mut self, children: Vec<FileNode>) {
        self.children = children;
    }

    /// Mark folder expanded/collapsed.
    pub(crate) fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
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

/// Build explorer root label from file system path.
pub(crate) fn root_label(path: &Path) -> String {
    let display = path.display();
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{display}"))
}

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

    use super::{FileNode, root_label};

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

    #[test]
    fn given_root_path_without_file_name_when_label_requested_then_uses_display()
     {
        assert_eq!(root_label(PathBuf::from("/").as_path()), "/");
    }
}
