use iced::Task;
use otty_app_explorer::{ExplorerIntent, ExplorerOutcome};
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Feature;

use super::notifications;
use crate::app::App;
use crate::effects;
use crate::message::AppEvent;

/// Give a Files section intent to the explorer and read what it reports.
///
/// The folder of the active terminal reaches the explorer as
/// [`ExplorerIntent::SyncRoot`]: the terminal feature (issue 11) sends it
/// through here when the active tab or pane changes, as the old app did.
/// Until then the explorer follows no folder.
pub(crate) fn deliver(app: &mut App, intent: ExplorerIntent) -> Task<AppEvent> {
    let outcomes = app.explorer.reduce(intent, ());

    let tasks: Vec<Task<AppEvent>> = outcomes
        .into_iter()
        .map(|outcome| read(app, outcome))
        .collect();

    Task::batch(tasks)
}

fn read(app: &mut App, outcome: ExplorerOutcome) -> Task<AppEvent> {
    match outcome {
        ExplorerOutcome::LoadRoot { root } => {
            effects::explorer::load_root(root)
        },
        ExplorerOutcome::LoadFolder { path, directory } => {
            effects::explorer::load_pressed_folder(path, directory)
        },
        ExplorerOutcome::ReloadFolder { directory } => {
            effects::explorer::reload_folder(directory)
        },
        // The file opens in a command tab with the saved editor of Settings
        // (issue 09): the tabs feature (issue 10) opens the tab and the
        // terminal feature (issue 11) runs it with its terminal settings.
        // Until they move in, nothing opens.
        ExplorerOutcome::OpenFile { file_path: _ } => Task::none(),
        ExplorerOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_explorer::FileNode;
    use otty_app_i18n as i18n;

    use super::*;
    use crate::testing::window::app;
    use crate::view::view;

    const ROOT: &str = "/tmp/otty-explorer-dispatch";

    /// An app whose explorer follows [`ROOT`] and shows its folder `src`.
    fn app_following_root() -> App {
        let mut app = app();
        let root = PathBuf::from(ROOT);
        let _read =
            deliver(&mut app, ExplorerIntent::SyncRoot { cwd: root.clone() });
        let _none = deliver(
            &mut app,
            ExplorerIntent::RootLoaded {
                root: root.clone(),
                nodes: vec![
                    FileNode::new(String::from("src"), root.join("src"), true),
                    FileNode::new(
                        String::from("notes.txt"),
                        root.join("notes.txt"),
                        false,
                    ),
                ],
            },
        );
        app
    }

    /// Following a new folder starts reading it.
    #[test]
    fn a_followed_folder_is_read_in_the_background() {
        let mut app = app();

        let task = deliver(
            &mut app,
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from(ROOT),
            },
        );

        assert_eq!(task.units(), 1);
        assert_eq!(app.explorer.watched_folders(), vec![PathBuf::from(ROOT)]);
    }

    #[test]
    fn a_pressed_closed_folder_is_read_in_the_background() {
        let mut app = app_following_root();

        let task = deliver(
            &mut app,
            ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            },
        );

        assert_eq!(task.units(), 1);
    }

    #[test]
    fn a_changed_watched_folder_is_read_again() {
        let mut app = app_following_root();

        let task = deliver(
            &mut app,
            ExplorerIntent::DirectoryChanged {
                directory: PathBuf::from(ROOT),
            },
        );

        assert_eq!(task.units(), 1);
    }

    /// A folder the user pressed open that could not be read shows a toast
    /// in the same update.
    #[test]
    fn a_pressed_folder_that_cannot_be_read_shows_a_toast() {
        let mut app = app();

        let _none = deliver(
            &mut app,
            ExplorerIntent::FolderLoadFailed {
                path: vec![String::from("src")],
                message: String::from("no access"),
            },
        );

        let text = i18n::folder_open_failed("src", "no access");
        let mut ui = simulator(view(&app));
        assert!(ui.find(text).is_ok());
        assert_eq!(app.notifications.unseen_count(), 1);
    }

    /// A pressed file opens in a command tab with the saved editor; until
    /// the tabs and the terminal move in (issues 10 and 11) nothing opens.
    #[test]
    fn a_pressed_file_opens_nothing_yet() {
        let mut app = app_following_root();

        let task = deliver(
            &mut app,
            ExplorerIntent::NodePressed {
                path: vec![String::from("notes.txt")],
            },
        );

        assert_eq!(task.units(), 0);
    }
}
