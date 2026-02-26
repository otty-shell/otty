use std::path::PathBuf;

use iced::Task;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};
use otty_ui_tree::TreePath;

use super::errors::ExplorerError;
use super::event::ExplorerEvent;
use super::model::{ExplorerLoadTarget, FileNode};
use super::services::read_dir_nodes;
use super::state::ExplorerState;
use crate::app::Event as AppEvent;
use crate::features::terminal::terminal_settings_for_session;

/// Runtime context required by explorer feature reducer.
pub(crate) struct ExplorerCtx<'a> {
    /// Pre-resolved CWD of the active shell terminal, used for explorer sync.
    pub(crate) active_shell_cwd: Option<PathBuf>,
    pub(crate) terminal_settings: &'a Settings,
    pub(crate) editor_command: &'a str,
}

/// Explorer feature root that owns explorer state and reduction logic.
#[derive(Debug)]
pub(crate) struct ExplorerFeature {
    state: ExplorerState,
}

impl ExplorerFeature {
    /// Construct explorer feature with default state.
    pub(crate) fn new() -> Self {
        Self {
            state: ExplorerState::new(),
        }
    }

    /// Return current root label used in explorer header.
    pub(crate) fn root_label(&self) -> Option<&str> {
        self.state.root_label()
    }

    /// Return root tree entries.
    pub(crate) fn tree(&self) -> &[FileNode] {
        self.state.tree()
    }

    /// Return selected tree path.
    pub(crate) fn selected_path(&self) -> Option<&TreePath> {
        self.state.selected_path()
    }

    /// Return hovered tree path.
    pub(crate) fn hovered_path(&self) -> Option<&TreePath> {
        self.state.hovered_path()
    }

    fn reduce_node_pressed(
        &mut self,
        ctx: &ExplorerCtx<'_>,
        path: TreePath,
    ) -> Task<AppEvent> {
        self.state.set_selected_path(Some(path.clone()));

        if self.state.node_is_folder(&path).unwrap_or(false) {
            let Some(load_path) = self.state.toggle_folder(&path) else {
                return Task::none();
            };

            return request_load_folder(path, load_path);
        }

        let Some(file_path) = self.state.node_path(&path) else {
            return Task::none();
        };

        open_file_in_editor(
            ctx.terminal_settings,
            ctx.editor_command,
            file_path,
        )
    }

    fn reduce_sync_from_active_terminal(
        &mut self,
        ctx: &ExplorerCtx<'_>,
    ) -> Task<AppEvent> {
        let Some(root) = ctx.active_shell_cwd.clone() else {
            return Task::none();
        };
        if !self.state.set_root(Some(root.clone())) {
            return Task::none();
        }

        request_load_root(root)
    }
}

impl ExplorerFeature {
    /// Reduce an explorer event into state updates and routed app tasks.
    pub(crate) fn reduce(
        &mut self,
        event: ExplorerEvent,
        ctx: &ExplorerCtx<'_>,
    ) -> Task<AppEvent> {
        match event {
            ExplorerEvent::NodePressed { path } => {
                self.reduce_node_pressed(ctx, path)
            },
            ExplorerEvent::NodeHovered { path } => {
                self.state.set_hovered_path(path);
                Task::none()
            },
            ExplorerEvent::SyncFromActiveTerminal => {
                self.reduce_sync_from_active_terminal(ctx)
            },
            ExplorerEvent::RootLoaded { root, nodes } => {
                let _ = self.state.apply_root_nodes(&root, nodes);
                Task::none()
            },
            ExplorerEvent::FolderLoaded { path, nodes } => {
                let _ = self.state.apply_folder_nodes(&path, nodes);
                Task::none()
            },
            ExplorerEvent::LoadFailed { target, message } => {
                log::warn!("explorer failed to load {target}: {message}");
                Task::none()
            },
        }
    }
}

fn request_load_root(root: PathBuf) -> Task<AppEvent> {
    Task::perform(
        async move {
            let loaded = read_dir_nodes(&root.clone());
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
            let loaded = read_dir_nodes(&directory.clone());
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
    let settings = terminal_settings_for_session(terminal_settings, session);

    let file_display = file_path.display();
    let title = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("{file_display}"));

    Task::done(AppEvent::OpenCommandTerminalTab {
        title,
        settings: Box::new(settings),
    })
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

    use otty_ui_term::settings::Settings;

    use super::{ExplorerCtx, ExplorerEvent, ExplorerFeature};
    use crate::features::explorer::FileNode;
    use crate::features::explorer::model::ExplorerLoadTarget;

    fn ctx<'a>(
        active_shell_cwd: Option<PathBuf>,
        terminal_settings: &'a Settings,
    ) -> ExplorerCtx<'a> {
        ExplorerCtx {
            active_shell_cwd,
            terminal_settings,
            editor_command: "nano",
        }
    }

    #[test]
    fn given_node_hover_event_when_reduced_then_hover_path_is_updated() {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let _task = feature.reduce(
            ExplorerEvent::NodeHovered {
                path: Some(vec![String::from("src")]),
            },
            &ctx(None, &settings),
        );

        let hovered = feature.hovered_path().expect("hover path should be set");
        assert_eq!(hovered, &vec![String::from("src")]);
    }

    #[test]
    fn given_root_loaded_event_when_root_matches_then_tree_is_applied() {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let root = PathBuf::from("/tmp");
        let _ = feature.state.set_root(Some(root.clone()));

        let _task = feature.reduce(
            ExplorerEvent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            },
            &ctx(None, &settings),
        );

        assert_eq!(feature.tree().len(), 1);
        assert_eq!(feature.tree()[0].name(), "main.rs");
    }

    #[test]
    fn given_root_loaded_event_for_stale_root_when_reduced_then_tree_is_ignored()
     {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let _ = feature.state.set_root(Some(PathBuf::from("/tmp/a")));

        let _task = feature.reduce(
            ExplorerEvent::RootLoaded {
                root: PathBuf::from("/tmp/b"),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    PathBuf::from("/tmp/b/main.rs"),
                    false,
                )],
            },
            &ctx(None, &settings),
        );

        assert!(feature.tree().is_empty());
    }

    #[test]
    fn given_sync_from_active_terminal_without_cwd_when_reduced_then_state_is_unchanged()
     {
        let mut feature = ExplorerFeature::new();
        let root = PathBuf::from("/tmp/original");
        let _ = feature.state.set_root(Some(root.clone()));
        let _ = feature.state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let settings = Settings::default();
        let _task = feature.reduce(
            ExplorerEvent::SyncFromActiveTerminal,
            &ctx(None, &settings),
        );

        assert_eq!(feature.root_label(), Some("original"));
        assert_eq!(feature.tree().len(), 1);
        assert_eq!(feature.tree()[0].name(), "src");
    }

    #[test]
    fn given_sync_from_active_terminal_with_shell_cwd_when_reduced_then_root_is_updated()
     {
        let mut feature = ExplorerFeature::new();
        let root = PathBuf::from("/tmp/original");
        let _ = feature.state.set_root(Some(root.clone()));
        let _ = feature.state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let shell_root = PathBuf::from("/tmp/otty-shell-root");
        let settings = Settings::default();
        let _task = feature.reduce(
            ExplorerEvent::SyncFromActiveTerminal,
            &ctx(Some(shell_root), &settings),
        );

        assert_eq!(feature.root_label(), Some("otty-shell-root"));
        assert!(feature.tree().is_empty());
    }

    #[test]
    fn given_folder_loaded_event_when_reduced_then_folder_children_are_applied()
    {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let root = PathBuf::from("/tmp");
        let _ = feature.state.set_root(Some(root.clone()));
        let _ = feature.state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        let _ = feature.state.toggle_folder(&[String::from("src")]);

        let _task = feature.reduce(
            ExplorerEvent::FolderLoaded {
                path: vec![String::from("src")],
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("src/main.rs"),
                    false,
                )],
            },
            &ctx(None, &settings),
        );

        assert_eq!(
            feature
                .state
                .node_path(&[String::from("src"), String::from("main.rs")]),
            Some(root.join("src/main.rs")),
        );
    }

    #[test]
    fn given_folder_node_when_pressed_then_selection_is_set() {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let root = PathBuf::from("/tmp");
        let _ = feature.state.set_root(Some(root.clone()));
        let _ = feature.state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let _task = feature.reduce(
            ExplorerEvent::NodePressed {
                path: vec![String::from("src")],
            },
            &ctx(None, &settings),
        );

        assert_eq!(feature.selected_path(), Some(&vec![String::from("src")]));
    }

    #[test]
    fn given_load_failed_event_when_reduced_then_state_is_not_mutated() {
        let mut feature = ExplorerFeature::new();
        let settings = Settings::default();

        let root = PathBuf::from("/tmp");
        let _ = feature.state.set_root(Some(root.clone()));
        let _ = feature.state.apply_root_nodes(
            &root,
            vec![FileNode::new(
                String::from("main.rs"),
                root.join("main.rs"),
                false,
            )],
        );

        let _task = feature.reduce(
            ExplorerEvent::LoadFailed {
                target: ExplorerLoadTarget::Root { root: root.clone() },
                message: String::from("boom"),
            },
            &ctx(None, &settings),
        );

        assert_eq!(feature.tree().len(), 1);
        assert_eq!(feature.tree()[0].name(), "main.rs");
    }
}
