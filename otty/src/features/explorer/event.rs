use std::path::PathBuf;

use iced::Task;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};
use otty_ui_tree::TreePath;

use crate::app::Event as AppEvent;
use crate::features::tab::{TabEvent, TabOpenRequest};
use crate::features::terminal::{self, shell_cwd_for_active_tab};
use crate::state::State;

use super::errors::ExplorerError;
use super::model::FileNode;
use super::services::load_directory_nodes;

/// Events emitted by explorer UI and async services.
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEvent {
    NodePressed {
        path: TreePath,
    },
    NodeHovered {
        path: Option<TreePath>,
    },
    SyncFromActiveTerminal,
    RootLoaded {
        root: PathBuf,
        nodes: Vec<FileNode>,
    },
    FolderLoaded {
        path: TreePath,
        nodes: Vec<FileNode>,
    },
    LoadFailed {
        target: ExplorerLoadTarget,
        message: String,
    },
}

/// Runtime dependencies used by explorer reducer.
pub(crate) struct ExplorerDeps<'a> {
    pub(crate) terminal_settings: &'a Settings,
    pub(crate) editor_command: &'a str,
}

#[derive(Debug, Clone)]
pub(crate) enum ExplorerLoadTarget {
    Root { root: PathBuf },
    Folder { path: TreePath, directory: PathBuf },
}

/// Handle explorer events and trigger side effects.
pub(crate) fn explorer_reducer(
    state: &mut State,
    deps: ExplorerDeps<'_>,
    event: ExplorerEvent,
) -> Task<AppEvent> {
    match event {
        ExplorerEvent::NodePressed { path } => {
            reduce_node_pressed(state, deps, path)
        },
        ExplorerEvent::NodeHovered { path } => {
            state.explorer.set_hovered_path(path);
            Task::none()
        },
        ExplorerEvent::SyncFromActiveTerminal => {
            reduce_sync_from_active_terminal(state)
        },
        ExplorerEvent::RootLoaded { root, nodes } => {
            let _ = state.explorer.apply_root_nodes(&root, nodes);
            Task::none()
        },
        ExplorerEvent::FolderLoaded { path, nodes } => {
            let _ = state.explorer.apply_folder_nodes(&path, nodes);
            Task::none()
        },
        ExplorerEvent::LoadFailed { target, message } => {
            let target_description = describe_load_target(&target);
            log::warn!(
                "explorer failed to load {target_description}: {message}"
            );
            Task::none()
        },
    }
}

fn describe_load_target(target: &ExplorerLoadTarget) -> String {
    match target {
        ExplorerLoadTarget::Root { root } => {
            let display = root.display();
            format!("root directory {display}")
        },
        ExplorerLoadTarget::Folder { path, directory } => {
            let directory_display = directory.display();
            format!("folder path {:?} from directory {directory_display}", path)
        },
    }
}

fn reduce_node_pressed(
    state: &mut State,
    deps: ExplorerDeps<'_>,
    path: TreePath,
) -> Task<AppEvent> {
    state.explorer.set_selected_path(Some(path.clone()));

    if state.explorer.node_is_folder(&path).unwrap_or(false) {
        let Some(load_path) = state.explorer.toggle_folder(&path) else {
            return Task::none();
        };

        return request_load_folder(path, load_path);
    }

    let Some(file_path) = state.explorer.node_path(&path) else {
        return Task::none();
    };

    open_file_in_editor(deps.terminal_settings, deps.editor_command, file_path)
}

fn reduce_sync_from_active_terminal(state: &mut State) -> Task<AppEvent> {
    let Some(root) = shell_cwd_for_active_tab(state) else {
        return Task::none();
    };
    if !state.explorer.set_root(Some(root.clone())) {
        return Task::none();
    }

    request_load_root(root)
}

fn request_load_root(root: PathBuf) -> Task<AppEvent> {
    Task::perform(
        async move {
            let loaded = load_directory_nodes(root.clone());
            (root, loaded)
        },
        |(root, result)| match result {
            Ok(nodes) => {
                AppEvent::Explorer(ExplorerEvent::RootLoaded { root, nodes })
            },
            Err(err) => AppEvent::Explorer(ExplorerEvent::LoadFailed {
                target: ExplorerLoadTarget::Root { root },
                message: format!("{err}"),
            }),
        },
    )
}

fn request_load_folder(path: TreePath, directory: PathBuf) -> Task<AppEvent> {
    Task::perform(
        async move {
            let loaded = load_directory_nodes(directory.clone());
            (path, directory, loaded)
        },
        |(path, directory, result)| match result {
            Ok(nodes) => {
                AppEvent::Explorer(ExplorerEvent::FolderLoaded { path, nodes })
            },
            Err(err) => AppEvent::Explorer(ExplorerEvent::LoadFailed {
                target: ExplorerLoadTarget::Folder { path, directory },
                message: format!("{err}"),
            }),
        },
    )
}

fn open_file_in_editor(
    terminal_settings: &Settings,
    editor_command: &str,
    file_path: PathBuf,
) -> Task<AppEvent> {
    let editor_raw = editor_command.trim();
    let (program, mut args) = match parse_command_line(editor_raw) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return Task::none();
        },
    };

    let file_arg = file_path.to_string_lossy().into_owned();
    args.push(file_arg);

    let mut options = LocalSessionOptions::default()
        .with_program(&program)
        .with_args(args);

    if let Some(parent) = file_path.parent() {
        options = options.with_working_directory(parent.into());
    }

    let session = SessionKind::from_local_options(options);
    let settings =
        terminal::terminal_settings_for_session(terminal_settings, session);

    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    Task::done(AppEvent::Tab(TabEvent::NewTab {
        request: TabOpenRequest::CommandTerminal {
            title,
            settings: Box::new(settings),
        },
    }))
}

fn parse_command_line(
    input: &str,
) -> Result<(String, Vec<String>), ExplorerError> {
    let parts = shell_words::split(input)?;
    let Some((program, args)) = parts.split_first() else {
        return Err(ExplorerError::EmptyEditorCommand);
    };

    Ok((program.clone(), args.to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};

    use super::{ExplorerDeps, ExplorerEvent, explorer_reducer};
    use crate::features::explorer::FileNode;
    use crate::features::tab::{TabContent, TabItem};
    use crate::features::terminal::{TerminalKind, TerminalState};
    use crate::state::State;

    #[cfg(unix)]
    const VALID_SHELL_PATH: &str = "/bin/sh";
    #[cfg(target_os = "windows")]
    const VALID_SHELL_PATH: &str = "cmd.exe";

    fn deps() -> ExplorerDeps<'static> {
        ExplorerDeps {
            terminal_settings: Box::leak(Box::new(Settings::default())),
            editor_command: "nano",
        }
    }

    fn settings_with_program(program: &str) -> Settings {
        let mut settings = Settings::default();
        settings.backend = settings.backend.clone().with_session(
            SessionKind::from_local_options(
                LocalSessionOptions::default().with_program(program),
            ),
        );
        settings
    }

    fn insert_command_tab(
        state: &mut State,
        tab_id: u64,
        terminal_id: u64,
        settings: Settings,
    ) {
        let (mut terminal, _task) = TerminalState::new(
            tab_id,
            String::from("Command"),
            terminal_id,
            settings,
            TerminalKind::Command,
        )
        .expect("terminal should initialize");
        terminal.set_grid_size(state.pane_grid_size());
        state.register_terminal_for_tab(terminal_id, tab_id);
        state.tab.insert(
            tab_id,
            TabItem {
                id: tab_id,
                title: terminal.title().to_string(),
                content: TabContent::Terminal(Box::new(terminal)),
            },
        );
        state.tab.activate(Some(tab_id));
    }

    #[test]
    fn given_node_hover_event_when_reduced_then_hover_path_is_updated() {
        let mut state = State::default();

        let _task = explorer_reducer(
            &mut state,
            deps(),
            ExplorerEvent::NodeHovered {
                path: Some(vec![String::from("src")]),
            },
        );

        let hovered = state
            .explorer
            .hovered_path()
            .expect("hover path should be set");
        assert_eq!(hovered, &vec![String::from("src")]);
    }

    #[test]
    fn given_root_loaded_event_when_root_matches_then_tree_is_applied() {
        let mut state = State::default();
        let root = PathBuf::from("/tmp");
        let _ = state.explorer.set_root(Some(root.clone()));

        let _task = explorer_reducer(
            &mut state,
            deps(),
            ExplorerEvent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            },
        );

        assert_eq!(state.explorer.tree().len(), 1);
        assert_eq!(state.explorer.tree()[0].name(), "main.rs");
    }

    #[test]
    fn given_root_loaded_event_for_stale_root_when_reduced_then_tree_is_ignored()
     {
        let mut state = State::default();
        let _ = state.explorer.set_root(Some(PathBuf::from("/tmp/a")));

        let _task = explorer_reducer(
            &mut state,
            deps(),
            ExplorerEvent::RootLoaded {
                root: PathBuf::from("/tmp/b"),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    PathBuf::from("/tmp/b/main.rs"),
                    false,
                )],
            },
        );

        assert!(state.explorer.tree().is_empty());
    }

    #[test]
    fn given_active_command_tab_when_sync_from_active_then_explorer_root_is_preserved()
     {
        let mut state = State::default();
        let root = std::env::temp_dir().join("otty-shell-root");
        let _ = state.explorer.set_root(Some(root.clone()));
        let _ = state.explorer.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        insert_command_tab(
            &mut state,
            1,
            10,
            settings_with_program(VALID_SHELL_PATH),
        );

        let _task = explorer_reducer(
            &mut state,
            deps(),
            ExplorerEvent::SyncFromActiveTerminal,
        );

        assert_eq!(state.explorer.root_label(), Some("otty-shell-root"));
        assert_eq!(state.explorer.tree().len(), 1);
        assert_eq!(state.explorer.tree()[0].name(), "src");
    }
}
