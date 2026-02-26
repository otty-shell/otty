use std::path::PathBuf;

use iced::Task;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind, Settings};
use otty_ui_tree::TreePath;

use super::command::ExplorerCommand;
use super::errors::ExplorerError;
use super::event::ExplorerEffectEvent;
use super::model::FileNode;
use super::state::ExplorerState;
use crate::widgets::terminal::terminal_settings_for_session;

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
    ) -> Task<ExplorerEffectEvent> {
        self.state.set_selected_path(Some(path.clone()));

        if self.state.node_is_folder(&path).unwrap_or(false) {
            let Some(load_path) = self.state.toggle_folder(&path) else {
                return Task::none();
            };

            return Task::done(ExplorerEffectEvent::LoadFolderRequested {
                path,
                directory: load_path,
            });
        }

        let Some(file_path) = self.state.node_path(&path) else {
            return Task::none();
        };

        let Some(effect) = open_file_in_editor(
            ctx.terminal_settings,
            ctx.editor_command,
            file_path,
        ) else {
            return Task::none();
        };

        Task::done(effect)
    }

    fn reduce_sync_from_active_terminal(
        &mut self,
        ctx: &ExplorerCtx<'_>,
    ) -> Task<ExplorerEffectEvent> {
        let Some(root) = ctx.active_shell_cwd.clone() else {
            return Task::none();
        };
        if !self.state.set_root(Some(root.clone())) {
            return Task::none();
        }

        Task::done(ExplorerEffectEvent::LoadRootRequested { root })
    }
}

impl ExplorerFeature {
    /// Reduce an explorer command into state updates and side-effect events.
    pub(crate) fn reduce(
        &mut self,
        command: ExplorerCommand,
        ctx: &ExplorerCtx<'_>,
    ) -> Task<ExplorerEffectEvent> {
        match command {
            ExplorerCommand::NodePressed { path } => {
                self.reduce_node_pressed(ctx, path)
            },
            ExplorerCommand::NodeHovered { path } => {
                self.state.set_hovered_path(path);
                Task::none()
            },
            ExplorerCommand::SyncFromActiveTerminal => {
                self.reduce_sync_from_active_terminal(ctx)
            },
            ExplorerCommand::RootLoaded { root, nodes } => {
                let _ = self.state.apply_root_nodes(&root, nodes);
                Task::none()
            },
            ExplorerCommand::FolderLoaded { path, nodes } => {
                let _ = self.state.apply_folder_nodes(&path, nodes);
                Task::none()
            },
            ExplorerCommand::LoadFailed { target, message } => {
                log::warn!("explorer failed to load {target}: {message}");
                Task::none()
            },
        }
    }
}

fn open_file_in_editor(
    terminal_settings: &Settings,
    editor_command: &str,
    file_path: PathBuf,
) -> Option<ExplorerEffectEvent> {
    let editor_raw = editor_command.trim();
    let (program, mut args) = match parse_command_line(editor_raw) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!("default editor parse failed: {err}");
            return None;
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

    Some(ExplorerEffectEvent::OpenCommandTerminalTab {
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

    use super::{ExplorerCtx, ExplorerFeature};
    use crate::widgets::explorer::model::ExplorerLoadTarget;
    use crate::widgets::explorer::{ExplorerCommand, FileNode};

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
            ExplorerCommand::NodeHovered {
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
            ExplorerCommand::RootLoaded {
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
            ExplorerCommand::RootLoaded {
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
            ExplorerCommand::SyncFromActiveTerminal,
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
            ExplorerCommand::SyncFromActiveTerminal,
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
            ExplorerCommand::FolderLoaded {
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
            ExplorerCommand::NodePressed {
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
            ExplorerCommand::LoadFailed {
                target: ExplorerLoadTarget::Root { root: root.clone() },
                message: String::from("boom"),
            },
            &ctx(None, &settings),
        );

        assert_eq!(feature.tree().len(), 1);
        assert_eq!(feature.tree()[0].name(), "main.rs");
    }
}
