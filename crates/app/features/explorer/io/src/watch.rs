//! The file system watch of the explorer: one long-running stream over the
//! folders the explorer asks to watch that reports which of them changed.

use std::collections::BTreeSet;
use std::path::PathBuf;

use iced::Subscription;
use iced::futures::channel::mpsc;
use iced::futures::stream::BoxStream;
use iced::futures::{FutureExt, SinkExt, StreamExt};
use notify::{RecursiveMode, Watcher};
use otty_app_explorer::ExplorerIntent;

use crate::changed_folders::{
    ExplorerWatchDirectories, WatchedDirectories, collect_changed_directories,
};

const WATCH_CHANNEL_SIZE: usize = 100;

/// Watch `directories`, the folders the explorer asks to watch
/// ([`otty_app_explorer::Explorer::watched_folders`]), for changes on disk;
/// each change comes back as [`ExplorerIntent::DirectoryChanged`]. No folder
/// watches nothing.
///
/// # Examples
///
/// ```
/// use otty_app_explorer_io::watch_folders;
///
/// assert_eq!(watch_folders(Vec::new()).units(), 0);
/// ```
pub fn watch_folders(
    directories: Vec<PathBuf>,
) -> Subscription<ExplorerIntent> {
    let directories = ExplorerWatchDirectories::new(directories);
    if directories.is_empty() {
        return Subscription::none();
    }

    Subscription::run_with(directories, explorer_watch_stream)
}

fn explorer_watch_stream(
    data: &ExplorerWatchDirectories,
) -> BoxStream<'static, ExplorerIntent> {
    let directories = data.directories().to_vec();

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
                    let event = ExplorerIntent::DirectoryChanged { directory };
                    if output.send(event).await.is_err() {
                        return;
                    }
                }
            }
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::watch_folders;

    #[test]
    fn given_empty_directories_when_subscription_created_then_no_units_are_registered()
     {
        let subscription = watch_folders(Vec::new());

        assert_eq!(subscription.units(), 0);
    }

    #[test]
    fn given_watch_directories_when_subscription_created_then_one_unit_is_registered()
     {
        let subscription = watch_folders(vec![PathBuf::from("/tmp/project")]);

        assert_eq!(subscription.units(), 1);
    }
}
