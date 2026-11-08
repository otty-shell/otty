//! Which loaded explorer folders a file system event changed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use notify::EventKind;
use notify::event::ModifyKind;

/// The loaded folders a watch covers, sorted and without repeats, so the
/// same folders always name the same watch.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct ExplorerWatchDirectories {
    directories: Vec<PathBuf>,
}

impl ExplorerWatchDirectories {
    /// Return the watched folders, sorted.
    pub(super) fn directories(&self) -> &[PathBuf] {
        &self.directories
    }

    /// Return whether there is no folder to watch.
    pub(super) fn is_empty(&self) -> bool {
        self.directories.is_empty()
    }

    /// Normalize `directories`: sort them and drop repeats.
    pub(super) fn new(mut directories: Vec<PathBuf>) -> Self {
        directories.sort();
        directories.dedup();

        Self { directories }
    }
}

/// The folders a running watch reports changes for.
#[derive(Debug)]
pub(super) struct WatchedDirectories {
    directories: BTreeSet<PathBuf>,
}

impl WatchedDirectories {
    fn contains(&self, path: &Path) -> bool {
        self.directories.contains(path)
    }

    /// Iterate over the watched folders, sorted.
    pub(super) fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.directories.iter()
    }

    /// Watch `directories`.
    pub(super) fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories: directories.into_iter().collect(),
        }
    }

    fn nearest_ancestor(&self, path: &Path) -> Option<PathBuf> {
        self.directories
            .iter()
            .filter(|directory| path.starts_with(directory))
            .max_by_key(|directory| directory.components().count())
            .cloned()
    }
}

/// Add to `changed_directories` the watched folders that the event in
/// `result` changed; a watcher error is logged and changes nothing.
pub(super) fn collect_changed_directories(
    result: notify::Result<notify::Event>,
    watched: &WatchedDirectories,
    changed_directories: &mut BTreeSet<PathBuf>,
) {
    let event = match result {
        Ok(event) => event,
        Err(err) => {
            log::warn!("explorer filesystem watcher event failed: {err}");
            return;
        },
    };

    for directory in changed_directories_from_event(&event, watched) {
        changed_directories.insert(directory);
    }
}

fn changed_directories_from_event(
    event: &notify::Event,
    watched: &WatchedDirectories,
) -> Vec<PathBuf> {
    if !event_may_change_tree(event.kind) {
        return Vec::new();
    }

    let mut directories = BTreeSet::new();
    for path in &event.paths {
        if let Some(parent) = path.parent()
            && watched.contains(parent)
        {
            directories.insert(parent.to_path_buf());
            continue;
        }

        if watched.contains(path) {
            directories.insert(path.clone());
            continue;
        }

        if let Some(directory) = watched.nearest_ancestor(path) {
            directories.insert(directory);
        }
    }

    directories.into_iter().collect()
}

fn event_may_change_tree(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Remove(_)
            | EventKind::Modify(ModifyKind::Any)
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Modify(ModifyKind::Other)
            | EventKind::Other
    )
}

#[cfg(test)]
mod tests {
    use notify::event::{
        AccessKind, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode,
    };

    use super::*;

    #[test]
    fn given_duplicate_watch_directories_when_normalized_then_sorted_unique_directories_are_kept()
     {
        let root = PathBuf::from("/tmp/project");
        let src = root.join("src");

        let directories = ExplorerWatchDirectories::new(vec![
            src.clone(),
            root.clone(),
            src.clone(),
        ]);

        assert_eq!(directories.directories, vec![root, src]);
    }

    #[test]
    fn given_create_event_for_child_when_mapped_then_parent_directory_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let src = root.join("src");
        let watched = WatchedDirectories::new(vec![root.clone(), src.clone()]);
        let event = notify::Event::new(EventKind::Create(CreateKind::File))
            .add_path(src.join("main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![src]);
    }

    #[test]
    fn given_create_event_for_root_child_when_mapped_then_root_is_returned() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Create(CreateKind::Folder))
            .add_path(root.join("src"));

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![root]);
    }

    #[test]
    fn given_rename_event_with_two_paths_when_mapped_then_both_parent_directories_are_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let src = root.join("src");
        let tests = root.join("tests");
        let watched = WatchedDirectories::new(vec![
            root.clone(),
            src.clone(),
            tests.clone(),
        ]);
        let event = notify::Event::new(EventKind::Modify(ModifyKind::Name(
            RenameMode::Both,
        )))
        .add_path(src.join("main.rs"))
        .add_path(tests.join("main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![src, tests]);
    }

    #[test]
    fn given_remove_event_for_watched_directory_when_mapped_then_parent_directory_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let src = root.join("src");
        let watched = WatchedDirectories::new(vec![root.clone(), src.clone()]);
        let event = notify::Event::new(EventKind::Remove(RemoveKind::Folder))
            .add_path(src);

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![root]);
    }

    #[test]
    fn given_nested_event_without_watched_parent_when_mapped_then_nearest_watched_ancestor_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Create(CreateKind::File))
            .add_path(root.join("src/nested/main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![root]);
    }

    #[test]
    fn given_event_for_watched_directory_when_mapped_then_same_directory_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Any).add_path(root.clone());

        let directories = changed_directories_from_event(&event, &watched);

        assert_eq!(directories, vec![root]);
    }

    #[test]
    fn given_event_outside_watched_tree_when_mapped_then_no_directory_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root]);
        let event = notify::Event::new(EventKind::Create(CreateKind::File))
            .add_path(PathBuf::from("/tmp/other/main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert!(directories.is_empty());
    }

    #[test]
    fn given_imprecise_modify_events_when_mapped_then_parent_directory_is_returned()
     {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let kinds = [
            EventKind::Modify(ModifyKind::Any),
            EventKind::Modify(ModifyKind::Other),
            EventKind::Other,
        ];

        for kind in kinds {
            let event = notify::Event::new(kind).add_path(root.join("main.rs"));

            let directories = changed_directories_from_event(&event, &watched);

            assert_eq!(directories, vec![root.clone()]);
        }
    }

    #[test]
    fn given_data_modify_event_when_mapped_then_no_directory_is_returned() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Modify(ModifyKind::Data(
            DataChange::Content,
        )))
        .add_path(root.join("main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert!(directories.is_empty());
    }

    #[test]
    fn given_metadata_modify_event_when_mapped_then_no_directory_is_returned() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Modify(
            ModifyKind::Metadata(notify::event::MetadataKind::Any),
        ))
        .add_path(root.join("main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert!(directories.is_empty());
    }

    #[test]
    fn given_access_event_when_mapped_then_no_directory_is_returned() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let event = notify::Event::new(EventKind::Access(AccessKind::Any))
            .add_path(root.join("main.rs"));

        let directories = changed_directories_from_event(&event, &watched);

        assert!(directories.is_empty());
    }

    #[test]
    fn given_notify_event_when_collected_then_directory_is_inserted() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root.clone()]);
        let mut directories = BTreeSet::new();
        let event = notify::Event::new(EventKind::Create(CreateKind::File))
            .add_path(root.join("main.rs"));

        collect_changed_directories(Ok(event), &watched, &mut directories);

        assert_eq!(directories.into_iter().collect::<Vec<_>>(), vec![root]);
    }

    #[test]
    fn given_notify_error_when_collected_then_no_directory_is_returned() {
        let root = PathBuf::from("/tmp/project");
        let watched = WatchedDirectories::new(vec![root]);
        let mut directories = BTreeSet::new();

        collect_changed_directories(
            Err(notify::Error::generic("boom")),
            &watched,
            &mut directories,
        );

        assert!(directories.is_empty());
    }
}
