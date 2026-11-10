use std::path::Path;

use iced::Task;
use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::tab::TabMachine;
use otty_app_explorer::{ExplorerIntent, ExplorerOutcome};
use otty_app_notifications::NotificationsIntent;
use otty_app_slots::Feature;
use otty_app_tabs::TabsIntent;

use super::{notifications, tabs, terminal};
use crate::app::App;
use crate::message::AppEvent;
use crate::{effects, terminal_settings};

/// Give a Files section intent to the explorer and read what it reports.
///
/// The folder of the active terminal reaches the explorer as
/// [`ExplorerIntent::SyncRoot`]: the terminal sends it through here when
/// the tab in front, its focused pane or the pane's folder changes.
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
        ExplorerOutcome::OpenFile { file_path } => open_file(app, &file_path),
        ExplorerOutcome::Failed(text) => {
            notifications::deliver(app, NotificationsIntent::Push(text))
        },
    }
}

/// Open `file_path` in a command tab with the saved editor. Nothing opens
/// when the saved editor command can not be read.
fn open_file(app: &mut App, file_path: &Path) -> Task<AppEvent> {
    let Some(intent) = open_file_intent(app, file_path) else {
        return Task::none();
    };

    tabs::deliver(app, intent)
}

/// The command tab that opens `file_path` with the saved editor,
/// `app.settings.terminal_editor()` (not an edit in the open Settings
/// dialog), on this computer; it closes when the editor ends. `None` when
/// the editor command can not be read.
fn open_file_intent(app: &App, file_path: &Path) -> Option<TabsIntent> {
    let editor = app.settings.terminal_editor().trim();
    let settings = terminal_settings::for_editor(
        editor,
        &terminal::base_settings(app),
        file_path,
    )?;
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| file_path.display().to_string());

    Some(TabsIntent::OpenCommandTab {
        title,
        settings: Box::new(settings),
        exit_behavior: ExitBehavior::Close,
        machine: TabMachine::ThisComputer,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_explorer::FileNode;
    use otty_app_i18n as i18n;
    use otty_app_settings::{SettingsIntent, SettingsLoad, SettingsLoadStatus};
    use otty_app_testing::temp_dir::TempDir;
    use otty_ui_term::settings::SessionKind;

    use super::*;
    use crate::dispatch::settings;
    use crate::testing::window::{
        TEST_SHELL_PATH, app, open_settings, test_settings,
    };
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

    /// A pressed file opens in a command tab named after it, with the saved
    /// editor, in the same update.
    #[test]
    fn a_pressed_file_opens_in_a_command_tab() {
        // The editor starts in the folder of the file, so it must exist.
        let folder = TempDir::new("open-file");
        let root = folder.path().to_path_buf();
        let mut app = app();
        let _read =
            deliver(&mut app, ExplorerIntent::SyncRoot { cwd: root.clone() });
        let _none = deliver(
            &mut app,
            ExplorerIntent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("notes.txt"),
                    root.join("notes.txt"),
                    false,
                )],
            },
        );
        let mut saved = test_settings();
        saved.set_terminal_editor(String::from(TEST_SHELL_PATH));
        let _none = settings::deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                saved,
                SettingsLoadStatus::Loaded,
            )),
        );

        let _open = deliver(
            &mut app,
            ExplorerIntent::NodePressed {
                path: vec![String::from("notes.txt")],
            },
        );

        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.tabs.active_tab_title(), Some("notes.txt"));
        assert_eq!(app.terminal.pane_count(), Some(1));
    }

    /// The program of the terminal a tab intent starts.
    fn program(intent: &TabsIntent) -> Option<&str> {
        let TabsIntent::OpenCommandTab { settings, .. } = intent else {
            return None;
        };

        match &settings.backend.session {
            SessionKind::Local(options) => Some(options.program().as_str()),
            SessionKind::Ssh(_) => None,
        }
    }

    /// The file opens with the saved editor, not with an edit in the open
    /// Settings dialog that is not saved yet.
    #[test]
    fn a_file_opens_in_the_saved_editor_not_an_unsaved_one() {
        let mut app = app();
        let mut saved = test_settings();
        saved.set_terminal_editor(String::from("otty-saved-editor"));
        let _none = settings::deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                saved,
                SettingsLoadStatus::Loaded,
            )),
        );
        open_settings(&mut app);
        let _none = settings::deliver(
            &mut app,
            SettingsIntent::EditorChanged(String::from("otty-unsaved-editor")),
        );

        let intent =
            open_file_intent(&app, Path::new("/tmp/project/notes.txt"))
                .expect("the saved editor command reads");

        assert_eq!(program(&intent), Some("otty-saved-editor"));
    }

    /// An editor command that can not be read opens nothing.
    #[test]
    fn an_unreadable_editor_command_opens_nothing() {
        let mut app = app();
        let mut saved = test_settings();
        saved.set_terminal_editor(String::from("nvim '"));
        let _none = settings::deliver(
            &mut app,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                saved,
                SettingsLoadStatus::Loaded,
            )),
        );

        let task = open_file(&mut app, Path::new("/tmp/project/notes.txt"));

        assert_eq!(task.units(), 0);
        assert_eq!(app.tabs.len(), 0);
    }
}
