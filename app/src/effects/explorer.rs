use std::path::PathBuf;

use iced::Task;

use super::notifications;
use crate::app::App;
use crate::domain::exit_behavior::ExitBehavior;
use crate::event::AppEvent;
use crate::services::editor_terminal_settings;
use crate::widgets::explorer::event::{
    ExplorerEffect, ExplorerEvent, ExplorerIntent,
};
use crate::widgets::explorer::services::read_dir_nodes;
use crate::widgets::explorer::types::{FileNode, TreePath};
use crate::widgets::tabs::event::{TabsEvent, TabsIntent};
use crate::widgets::tabs::types::TabMachine;

/// Run a Files panel effect: read folders in the background, open a file in
/// the saved editor, or tell the user about a failure.
pub(crate) fn run(app: &App, effect: ExplorerEffect) -> Task<AppEvent> {
    match effect {
        ExplorerEffect::LoadRootRequested { root } => {
            load_directory_async(root.clone(), move |nodes| {
                ExplorerIntent::RootLoaded {
                    root: root.clone(),
                    nodes,
                }
            })
        },
        ExplorerEffect::LoadFolderRequested { path, directory } => {
            load_pressed_folder(path, directory)
        },
        ExplorerEffect::ReloadDirectoryRequested { directory } => {
            load_directory_async(directory.clone(), move |nodes| {
                ExplorerIntent::DirectoryReloaded {
                    directory: directory.clone(),
                    nodes,
                }
            })
        },
        ExplorerEffect::OpenFileTerminalTab { file_path } => {
            open_file_terminal_tab(app, file_path)
        },
        ExplorerEffect::Failed(text) => notifications::push(text),
    }
}

/// Read the folder the user pressed open in the background. Unlike the
/// loads that follow the terminal or the disk, its failure comes back as
/// its own intent, so the widget can tell the user.
fn load_pressed_folder(path: TreePath, directory: PathBuf) -> Task<AppEvent> {
    Task::perform(
        async move { read_dir_nodes(&directory).map_err(|err| format!("{err}")) },
        move |result| {
            AppEvent::Explorer(ExplorerEvent::Intent(folder_loaded(
                path, result,
            )))
        },
    )
}

/// What a finished read of a pressed folder leads to: its files, or the
/// failure to tell the user about.
fn folder_loaded(
    path: TreePath,
    result: Result<Vec<FileNode>, String>,
) -> ExplorerIntent {
    match result {
        Ok(nodes) => ExplorerIntent::FolderLoaded { path, nodes },
        Err(message) => ExplorerIntent::FolderLoadFailed { path, message },
    }
}

/// Read a directory the user did not press in the background and produce a
/// completion event. Its failure is only logged.
fn load_directory_async<F>(dir: PathBuf, on_complete: F) -> Task<AppEvent>
where
    F: Fn(Vec<FileNode>) -> ExplorerIntent + Send + 'static,
{
    Task::perform(async move { read_dir_nodes(&dir) }, move |result| {
        match result {
            Ok(nodes) => {
                AppEvent::Explorer(ExplorerEvent::Intent(on_complete(nodes)))
            },
            Err(err) => AppEvent::Explorer(ExplorerEvent::Intent(
                ExplorerIntent::LoadFailed {
                    message: format!("{err}"),
                },
            )),
        }
    })
}

fn open_file_terminal_tab(app: &App, file_path: PathBuf) -> Task<AppEvent> {
    let Some(settings) = editor_terminal_settings(
        app.widgets.settings.terminal_editor().trim(),
        &app.terminal_settings,
        &file_path,
    ) else {
        return Task::none();
    };

    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    Task::done(AppEvent::Tabs(TabsEvent::Intent(
        TabsIntent::OpenCommandTab {
            title,
            settings: Box::new(settings),
            exit_behavior: ExitBehavior::Close,
            machine: TabMachine::ThisComputer,
        },
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced::Task;
    use iced::futures::StreamExt;
    use iced_test::runtime::Action;
    use iced_test::runtime::task::into_stream;
    use otty_ui_term::settings::SessionKind;

    use super::{folder_loaded, run};
    use crate::app::App;
    use crate::event::AppEvent;
    use crate::testing::task_outputs::collect_outputs;
    use crate::testing::temp_dir::TempDir;
    use crate::widgets::explorer::event::{
        ExplorerEffect, ExplorerEvent, ExplorerIntent,
    };
    use crate::widgets::explorer::types::FileNode;
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };
    use crate::widgets::settings::event::SettingsIntent;
    use crate::widgets::settings::types::{
        SettingsData, SettingsLoad, SettingsLoadStatus,
    };
    use crate::widgets::tabs::event::{TabsEvent, TabsIntent};

    /// Run a task that reads the disk on the runtime the app runs on and
    /// return the Files intents it sent.
    fn run_to_intents(task: Task<AppEvent>) -> Vec<ExplorerIntent> {
        let runtime = iced::executor::Default::new().expect("a tokio runtime");
        let stream = into_stream(task).expect("a task");

        let actions: Vec<Action<AppEvent>> = runtime.block_on(stream.collect());

        actions
            .into_iter()
            .filter_map(|action| match action {
                Action::Output(AppEvent::Explorer(ExplorerEvent::Intent(
                    intent,
                ))) => Some(intent),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_read_pressed_folder_gives_its_files() {
        let nodes = vec![FileNode::new(
            String::from("main.rs"),
            PathBuf::from("/tmp/project/src/main.rs"),
            false,
        )];

        let intent = folder_loaded(vec![String::from("src")], Ok(nodes));

        assert!(matches!(
            intent,
            ExplorerIntent::FolderLoaded { path, nodes }
                if path == [String::from("src")] && nodes.len() == 1
        ));
    }

    #[test]
    fn a_pressed_folder_that_cannot_be_read_gives_its_own_failure() {
        let intent = folder_loaded(
            vec![String::from("src")],
            Err(String::from("no access")),
        );

        assert!(matches!(
            intent,
            ExplorerIntent::FolderLoadFailed { path, message }
                if path == [String::from("src")] && message == "no access"
        ));
    }

    /// Only the load that a press started reports its own failure; the
    /// root load that follows the terminal keeps the log-only one.
    #[test]
    fn only_the_load_of_a_pressed_folder_reports_a_failure_to_tell() {
        let (app, _) = App::new();
        let root = TempDir::new("explorer-effects-missing");
        let missing = root.path().join("missing");

        let pressed = run_to_intents(run(
            &app,
            ExplorerEffect::LoadFolderRequested {
                path: vec![String::from("missing")],
                directory: missing.clone(),
            },
        ));
        let followed = run_to_intents(run(
            &app,
            ExplorerEffect::LoadRootRequested { root: missing },
        ));

        assert!(matches!(
            pressed.as_slice(),
            [ExplorerIntent::FolderLoadFailed { path, .. }]
                if path == &[String::from("missing")]
        ));
        assert!(matches!(
            followed.as_slice(),
            [ExplorerIntent::LoadFailed { .. }]
        ));
    }

    #[test]
    fn a_failure_becomes_a_notification() {
        let (app, _) = App::new();

        let events = collect_outputs(run(
            &app,
            ExplorerEffect::Failed(String::from(
                "Could not open the folder \"src\": no access",
            )),
        ));

        assert!(matches!(
            events.as_slice(),
            [AppEvent::Notifications(NotificationsEvent::Intent(
                NotificationsIntent::Push(text)
            ))] if text == "Could not open the folder \"src\": no access"
        ));
    }

    /// Owner, Q4: a file from Files opens in the saved editor, not in one
    /// typed in the open Settings popup and not saved yet.
    #[test]
    fn a_file_opens_in_the_saved_editor_not_an_unsaved_one() {
        let (mut app, _) = App::new();
        let mut saved = SettingsData::default();
        saved.set_terminal_editor(String::from("otty-saved-editor"));
        let _ = app.widgets.settings.reduce(SettingsIntent::ReloadLoaded(
            SettingsLoad::new(saved, SettingsLoadStatus::Loaded),
        ));
        let _ = app.widgets.settings.reduce(SettingsIntent::Open);
        let _ = app.widgets.settings.reduce(SettingsIntent::EditorChanged(
            String::from("otty-unsaved-editor"),
        ));

        let events = collect_outputs(run(
            &app,
            ExplorerEffect::OpenFileTerminalTab {
                file_path: PathBuf::from("/tmp/project/notes.txt"),
            },
        ));

        let programs: Vec<&str> = events
            .iter()
            .filter_map(|event| match event {
                AppEvent::Tabs(TabsEvent::Intent(
                    TabsIntent::OpenCommandTab { settings, .. },
                )) => match &settings.backend.session {
                    SessionKind::Local(options) => {
                        Some(options.program().as_str())
                    },
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(programs, vec!["otty-saved-editor"]);
    }

    #[test]
    fn given_reload_directory_requested_when_handled_then_load_task_is_emitted()
    {
        let (app, _) = App::new();

        let task = run(
            &app,
            ExplorerEffect::ReloadDirectoryRequested {
                directory: PathBuf::from("/tmp/project"),
            },
        );

        assert_eq!(task.units(), 1);
    }
}
