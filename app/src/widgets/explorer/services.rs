use std::path::Path;

use super::errors::ExplorerError;
use super::types::FileNode;

/// Build explorer root label from a file system path.
///
/// Returns the final path component, or the full display string for root paths.
pub(crate) fn root_label(path: &Path) -> String {
    let display = path.display();
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{display}"))
}

/// Load direct children for a file system directory.
///
/// Returns nodes sorted folders-first, then alphabetical.
pub(crate) fn read_dir_nodes(
    path: &Path,
) -> Result<Vec<FileNode>, ExplorerError> {
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

    nodes.sort();

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::testing::temp_dir::TempDir;

    #[test]
    fn given_directory_with_files_when_loaded_then_nodes_are_returned() {
        let root = TempDir::new("explorer-load-directory");
        fs::create_dir_all(root.path().join("b-dir"))
            .expect("folder should be created");
        fs::write(root.path().join("a.txt"), "ok")
            .expect("file should be created");

        let nodes = read_dir_nodes(root.path()).expect("directory should load");

        assert_eq!(nodes.len(), 2);
        assert!(nodes[0].is_folder());
        assert_eq!(nodes[1].name(), "a.txt");
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
