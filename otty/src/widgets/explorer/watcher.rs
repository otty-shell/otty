use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::stream::BoxStream;
use iced::futures::{FutureExt, SinkExt, StreamExt};
use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher};

use super::{ExplorerEvent, ExplorerIntent};

const WATCH_CHANNEL_SIZE: usize = 100;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ExplorerWatchDirectories {
    directories: Vec<PathBuf>,
}

impl ExplorerWatchDirectories {
    fn new(mut directories: Vec<PathBuf>) -> Self {
        directories.sort();
        directories.dedup();

        Self { directories }
    }

    fn is_empty(&self) -> bool {
        self.directories.is_empty()
    }
}

#[derive(Debug)]
struct WatchedDirectories {
    directories: BTreeSet<PathBuf>,
}

impl WatchedDirectories {
    fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories: directories.into_iter().collect(),
        }
    }

    fn contains(&self, path: &Path) -> bool {
        self.directories.contains(path)
    }

    fn iter(&self) -> impl Iterator<Item = &PathBuf> {
        self.directories.iter()
    }

    fn nearest_ancestor(&self, path: &Path) -> Option<PathBuf> {
        self.directories
            .iter()
            .filter(|directory| path.starts_with(directory))
            .max_by_key(|directory| directory.components().count())
            .cloned()
    }
}

/// Subscribe to filesystem changes for currently loaded explorer directories.
pub(super) fn subscription(
    directories: Vec<PathBuf>,
) -> Subscription<ExplorerEvent> {
    let directories = ExplorerWatchDirectories::new(directories);
    if directories.is_empty() {
        return Subscription::none();
    }

    Subscription::run_with(directories, explorer_watch_stream)
}

fn explorer_watch_stream(
    data: &ExplorerWatchDirectories,
) -> BoxStream<'static, ExplorerEvent> {
    let directories = data.directories.clone();

    Box::pin(iced::stream::channel(
        WATCH_CHANNEL_SIZE,
        async move |mut output| {
            let watched_directories = WatchedDirectories::new(directories);
            let (event_sender, mut event_receiver) =
                mpsc::unbounded::<notify::Result<notify::Event>>();

            let mut watcher = match notify::recommended_watcher(move |result| {
                let _ = event_sender.unbounded_send(result);
            }) {
                Ok(watcher) => watcher,
                Err(err) => {
                    log::warn!("explorer filesystem watcher failed: {err}");
                    return;
                },
            };

            for directory in watched_directories.iter() {
                if let Err(err) =
                    watcher.watch(directory, RecursiveMode::NonRecursive)
                {
                    let display = directory.display();
                    log::warn!("explorer failed to watch {display}: {err}");
                }
            }

            let _watcher = watcher;
            while let Some(result) = event_receiver.next().await {
                let mut changed_directories = BTreeSet::new();
                collect_changed_directories(
                    result,
                    &watched_directories,
                    &mut changed_directories,
                );

                while let Some(Some(result)) =
                    event_receiver.next().now_or_never()
                {
                    collect_changed_directories(
                        result,
                        &watched_directories,
                        &mut changed_directories,
                    );
                }

                for directory in changed_directories {
                    let event = ExplorerEvent::Intent(
                        ExplorerIntent::DirectoryChanged { directory },
                    );
                    if output.send(event).await.is_err() {
                        return;
                    }
                }
            }
        },
    ))
}

fn collect_changed_directories(
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
        if let Some(parent) = path.parent() {
            if watched.contains(parent) {
                directories.insert(parent.to_path_buf());
                continue;
            }
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
    fn given_empty_directories_when_subscription_created_then_no_units_are_registered()
     {
        let subscription = subscription(Vec::new());

        assert_eq!(subscription.units(), 0);
    }

    #[test]
    fn given_watch_directories_when_subscription_created_then_one_unit_is_registered()
     {
        let subscription = subscription(vec![PathBuf::from("/tmp/project")]);

        assert_eq!(subscription.units(), 1);
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
