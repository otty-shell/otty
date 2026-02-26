use std::path::Path;

use super::errors::ExplorerError;
use super::model::FileNode;

/// Build explorer root label from file system path.
pub(crate) fn root_label(path: &Path) -> String {
    let display = path.display();
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{display}"))
}

/// Load direct children for a file system directory.
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
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::read_dir_nodes;

    #[test]
    fn given_directory_with_files_when_loaded_then_nodes_are_returned() {
        let root = test_temp_dir("load_directory");
        fs::create_dir_all(root.join("b-dir"))
            .expect("folder should be created");
        fs::write(root.join("a.txt"), "ok").expect("file should be created");

        let nodes =
            read_dir_nodes(&root.clone()).expect("directory should load");

        assert_eq!(nodes.len(), 2);
        assert!(nodes[0].is_folder());
        assert_eq!(nodes[1].name(), "a.txt");

        fs::remove_dir_all(root).expect("test directory should be removed");
    }

    fn test_temp_dir(test_name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "otty-explorer-{test_name}-{stamp}-{}",
            std::process::id()
        ));

        fs::create_dir_all(&dir).expect("test directory should be created");
        dir
    }
}
