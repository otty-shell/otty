use std::path::PathBuf;

use iced::Task;

use super::command::ExplorerCommand;
use super::event::ExplorerEffect;
use super::state::ExplorerState;

/// Runtime context for the explorer reducer.
pub(crate) struct ExplorerCtx {
    /// Pre-resolved CWD of the active shell terminal.
    pub(crate) active_shell_cwd: Option<PathBuf>,
}

/// Reduce an explorer command into state updates and effects.
pub(crate) fn reduce(
    state: &mut ExplorerState,
    command: ExplorerCommand,
    _ctx: &ExplorerCtx,
) -> Task<ExplorerEffect> {
    match command {
        ExplorerCommand::NodePressed { path } => {
            reduce_node_pressed(state, path)
        },
        ExplorerCommand::NodeHovered { path } => {
            state.set_hovered_path(path);
            Task::none()
        },
        ExplorerCommand::SyncRoot { cwd } => reduce_sync_root(state, cwd),
        ExplorerCommand::RootLoaded { root, nodes } => {
            let _ = state.apply_root_nodes(&root, nodes);
            Task::none()
        },
        ExplorerCommand::FolderLoaded { path, nodes } => {
            let _ = state.apply_folder_nodes(&path, nodes);
            Task::none()
        },
        ExplorerCommand::LoadFailed { message } => {
            log::warn!("explorer load failed: {message}");
            Task::none()
        },
    }
}

/// Handle node press: select, toggle folders, open files.
fn reduce_node_pressed(
    state: &mut ExplorerState,
    path: Vec<String>,
) -> Task<ExplorerEffect> {
    state.set_selected_path(Some(path.clone()));

    if state.node_is_folder(&path).unwrap_or(false) {
        let Some(load_path) = state.toggle_folder(&path) else {
            return Task::none();
        };

        return Task::done(ExplorerEffect::LoadFolderRequested {
            path,
            directory: load_path,
        });
    }

    let Some(file_path) = state.node_path(&path) else {
        return Task::none();
    };

    Task::done(ExplorerEffect::OpenFileTerminalTab { file_path })
}

/// Handle root sync from the active terminal CWD.
fn reduce_sync_root(
    state: &mut ExplorerState,
    cwd: PathBuf,
) -> Task<ExplorerEffect> {
    if !state.set_root(Some(cwd.clone())) {
        return Task::none();
    }

    Task::done(ExplorerEffect::LoadRootRequested { root: cwd })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::widgets::explorer::model::FileNode;

    fn ctx(active_shell_cwd: Option<PathBuf>) -> ExplorerCtx {
        ExplorerCtx { active_shell_cwd }
    }

    #[test]
    fn given_node_hover_event_when_reduced_then_hover_path_is_updated() {
        let mut state = ExplorerState::default();

        let _task = reduce(
            &mut state,
            ExplorerCommand::NodeHovered {
                path: Some(vec![String::from("src")]),
            },
            &ctx(None),
        );

        let hovered = state.hovered_path().expect("hover path should be set");
        assert_eq!(hovered, &vec![String::from("src")]);
    }

    #[test]
    fn given_root_loaded_when_root_matches_then_tree_is_applied() {
        let mut state = ExplorerState::default();

        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));

        let _task = reduce(
            &mut state,
            ExplorerCommand::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            },
            &ctx(None),
        );

        assert_eq!(state.tree().len(), 1);
        assert_eq!(state.tree()[0].name(), "main.rs");
    }

    #[test]
    fn given_root_loaded_for_stale_root_when_reduced_then_tree_is_ignored() {
        let mut state = ExplorerState::default();

        state.set_root(Some(PathBuf::from("/tmp/a")));

        let _task = reduce(
            &mut state,
            ExplorerCommand::RootLoaded {
                root: PathBuf::from("/tmp/b"),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    PathBuf::from("/tmp/b/main.rs"),
                    false,
                )],
            },
            &ctx(None),
        );

        assert!(state.tree().is_empty());
    }

    #[test]
    fn given_sync_root_with_new_cwd_when_reduced_then_root_is_updated() {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp/original");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let _task = reduce(
            &mut state,
            ExplorerCommand::SyncRoot {
                cwd: PathBuf::from("/tmp/new-root"),
            },
            &ctx(None),
        );

        assert_eq!(state.root_label(), Some("new-root"));
        assert!(state.tree().is_empty());
    }

    #[test]
    fn given_sync_root_with_same_cwd_when_reduced_then_tree_is_unchanged() {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp/original");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let _task = reduce(
            &mut state,
            ExplorerCommand::SyncRoot {
                cwd: PathBuf::from("/tmp/original"),
            },
            &ctx(None),
        );

        assert_eq!(state.root_label(), Some("original"));
        assert_eq!(state.tree().len(), 1);
    }

    #[test]
    fn given_folder_node_when_pressed_then_selection_is_set() {
        let mut state = ExplorerState::default();

        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );

        let _task = reduce(
            &mut state,
            ExplorerCommand::NodePressed {
                path: vec![String::from("src")],
            },
            &ctx(None),
        );

        assert_eq!(state.selected_path(), Some(&vec![String::from("src")]));
    }

    #[test]
    fn given_load_failed_when_reduced_then_state_is_not_mutated() {
        let mut state = ExplorerState::default();

        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            &root,
            vec![FileNode::new(
                String::from("main.rs"),
                root.join("main.rs"),
                false,
            )],
        );

        let _task = reduce(
            &mut state,
            ExplorerCommand::LoadFailed {
                message: String::from("boom"),
            },
            &ctx(None),
        );

        assert_eq!(state.tree().len(), 1);
        assert_eq!(state.tree()[0].name(), "main.rs");
    }
}
