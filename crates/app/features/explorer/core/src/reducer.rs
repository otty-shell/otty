use std::path::PathBuf;

use otty_app_i18n as i18n;

use crate::ctx::ExplorerCtx;
use crate::disk_operation::DiskOperation;
use crate::event::{ExplorerIntent, ExplorerOutcome};
use crate::state::ExplorerState;
use crate::types::TreePath;
use crate::{drag, menu, naming};

/// Apply an intent to the state and return what the binary has to do. When
/// the last open overlay of Files closes (the context menu, the delete
/// dialog or the name field), it also reports
/// [`ExplorerOutcome::OverlayClosed`], last. A menu item that opens the name
/// field or the delete dialog in place of the menu reports nothing.
pub(crate) fn reduce(
    state: &mut ExplorerState,
    intent: ExplorerIntent,
    ctx: &ExplorerCtx,
) -> Vec<ExplorerOutcome> {
    let had_overlay = state.has_overlay();

    let mut outcomes = reduce_intent(state, intent, ctx);

    if had_overlay && !state.has_overlay() {
        outcomes.push(ExplorerOutcome::OverlayClosed);
    }

    outcomes
}

/// Open the row at `path` as a click does: a file in the editor, a folder
/// in the tree (or closed again when it is open).
pub(crate) fn open_node(
    state: &mut ExplorerState,
    path: TreePath,
) -> Vec<ExplorerOutcome> {
    state.set_selected_path(Some(path.clone()));

    if state.node_is_folder(&path).unwrap_or(false) {
        let Some(directory) = state.toggle_folder(&path) else {
            return Vec::new();
        };

        return vec![ExplorerOutcome::LoadFolder { path, directory }];
    }

    let Some(file_path) = state.node_path(&path) else {
        return Vec::new();
    };

    vec![ExplorerOutcome::OpenFile { file_path }]
}

/// Change the state for `intent` and return its outcomes, all but the
/// overlay closing.
fn reduce_intent(
    state: &mut ExplorerState,
    intent: ExplorerIntent,
    ctx: &ExplorerCtx,
) -> Vec<ExplorerOutcome> {
    match intent {
        ExplorerIntent::NodePressed { path } => {
            drag::hold(state, path);
        },
        ExplorerIntent::NodeReleased { path } => {
            return reduce_node_released(state, path);
        },
        ExplorerIntent::NodeRightClicked { path } => {
            return menu::open_on_node(state, path, ctx);
        },
        ExplorerIntent::NodeHovered { path } => {
            state.set_hovered_path(path);
            drag::follow_hover(state);
        },
        ExplorerIntent::BackgroundPressed => {
            state.set_selected_path(None);
        },
        ExplorerIntent::BackgroundReleased => {
            return drag::release_on_background(state);
        },
        ExplorerIntent::BackgroundRightClicked => {
            return menu::open_on_background(state, ctx);
        },
        ExplorerIntent::CursorMoved { position } => {
            state.set_cursor(position);
            drag::follow_cursor(state);
        },
        ExplorerIntent::ContextMenuDismiss => {
            state.set_context_menu(None);
        },
        ExplorerIntent::ContextMenuAction(action) => {
            return menu::act(state, action);
        },
        ExplorerIntent::InlineEditChanged(value) => {
            naming::update_value(state, value);
        },
        ExplorerIntent::InlineEditSubmit => return naming::submit(state),
        ExplorerIntent::CancelInlineEdit => state.set_inline_edit(None),
        ExplorerIntent::DeleteConfirmed => return menu::confirm_delete(state),
        ExplorerIntent::DeleteCancelled => state.set_pending_delete(None),
        ExplorerIntent::SyncRoot { cwd } => {
            return reduce_sync_root(state, cwd);
        },
        ExplorerIntent::RootLoaded { root, nodes } => {
            let _ = state.apply_root_nodes(&root, nodes);
        },
        ExplorerIntent::FolderLoaded { path, nodes } => {
            let _ = state.apply_folder_nodes(&path, nodes);
        },
        ExplorerIntent::DirectoryChanged { directory } => {
            return reduce_directory_changed(state, directory);
        },
        ExplorerIntent::DirectoryReloaded { directory, nodes } => {
            let _ = state.apply_directory_nodes(&directory, nodes);
        },
        ExplorerIntent::LoadFailed { message } => {
            log::warn!("explorer load failed: {message}");
        },
        ExplorerIntent::FolderLoadFailed { path, message } => {
            return vec![folder_load_failed(&path, &message)];
        },
        ExplorerIntent::DiskOperationDone { operation } => {
            return reduce_operation_done(state, &operation);
        },
        ExplorerIntent::DiskOperationFailed { operation, message } => {
            return vec![operation_failed(&operation, &message)];
        },
    }

    Vec::new()
}

/// A release on a row drops a dragged row, or opens the row when it was
/// pressed there: a click.
fn reduce_node_released(
    state: &mut ExplorerState,
    path: TreePath,
) -> Vec<ExplorerOutcome> {
    if let Some(dropped) = drag::finish(state) {
        return dropped;
    }

    let is_click = state.pressed_path() == Some(&path);
    state.set_pressed_path(None);
    if !is_click {
        return Vec::new();
    }

    open_node(state, path)
}

/// Handle root sync from the active terminal CWD.
fn reduce_sync_root(
    state: &mut ExplorerState,
    cwd: PathBuf,
) -> Vec<ExplorerOutcome> {
    if !state.set_root(Some(cwd.clone())) {
        return Vec::new();
    }

    vec![ExplorerOutcome::LoadRoot { root: cwd }]
}

/// Tell the user that the folder they pressed open could not be read, and
/// name it by the last part of its path.
fn folder_load_failed(path: &[String], message: &str) -> ExplorerOutcome {
    log::warn!("explorer folder load failed: {message}");

    let name = path.last().map_or("", String::as_str);
    let text = i18n::folder_open_failed(name, message);

    ExplorerOutcome::Failed(text)
}

/// Ask to read a changed folder again, if the tree shows it.
fn reduce_directory_changed(
    state: &ExplorerState,
    directory: PathBuf,
) -> Vec<ExplorerOutcome> {
    if !state.is_watched_directory(&directory) {
        return Vec::new();
    }

    vec![ExplorerOutcome::ReloadFolder { directory }]
}

/// Read again the folders a done operation altered that the tree shows, so
/// the tree shows the result at once, before the watch reports it.
fn reduce_operation_done(
    state: &ExplorerState,
    operation: &DiskOperation,
) -> Vec<ExplorerOutcome> {
    operation
        .changed_folders()
        .into_iter()
        .filter(|folder| state.is_watched_directory(folder))
        .map(|directory| ExplorerOutcome::ReloadFolder { directory })
        .collect()
}

/// Tell the user which entry could not be changed and why.
fn operation_failed(
    operation: &DiskOperation,
    message: &str,
) -> ExplorerOutcome {
    log::warn!("explorer disk operation failed: {message}");

    let text = i18n::files_change_failed(&operation.name(), message);

    ExplorerOutcome::Failed(text)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::event::ExplorerOutcome;
    use crate::testing::ctx;
    use crate::types::FileNode;

    /// A state that follows `root` and shows its folder `src`, closed.
    fn state_with_src(root: &PathBuf) -> ExplorerState {
        let mut state = ExplorerState::default();
        state.set_root(Some(root.clone()));
        let _ = state.apply_root_nodes(
            root,
            vec![FileNode::new(String::from("src"), root.join("src"), true)],
        );
        state
    }

    /// Press and release the row `name` at the top of the tree.
    fn click(state: &mut ExplorerState, name: &str) -> Vec<ExplorerOutcome> {
        let path = vec![String::from(name)];
        let _ = reduce(
            state,
            ExplorerIntent::NodePressed { path: path.clone() },
            &ctx(),
        );

        reduce(state, ExplorerIntent::NodeReleased { path }, &ctx())
    }

    /// A press alone selects the row and opens nothing: it may become a
    /// drag.
    #[test]
    fn a_press_alone_selects_the_row_and_opens_nothing() {
        let root = PathBuf::from("/tmp");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::NodePressed {
                path: vec![String::from("src")],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.selected_path(), Some(&vec![String::from("src")]));
        assert!(!state.tree()[0].is_expanded());
    }

    /// A release on another row than the pressed one is no click.
    #[test]
    fn a_release_on_another_row_opens_nothing() {
        let root = PathBuf::from("/tmp");
        let mut state = state_with_src(&root);
        let _ = reduce(
            &mut state,
            ExplorerIntent::NodePressed {
                path: vec![String::from("other")],
            },
            &ctx(),
        );

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: vec![String::from("src")],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }

    /// A folder the user pressed open that cannot be read tells the user,
    /// and names the folder.
    #[test]
    fn a_pressed_folder_that_cannot_be_read_tells_the_user() {
        let mut state = ExplorerState::default();

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::FolderLoadFailed {
                path: vec![String::from("src"), String::from("secret")],
                message: String::from("no access"),
            },
            &ctx(),
        );

        let expected = i18n::folder_open_failed("secret", "no access");
        assert_eq!(outcomes, vec![ExplorerOutcome::Failed(expected)]);
    }

    /// Loads the user did not ask for, such as following the folder of the
    /// terminal or a reload after a change on disk, only log a failure.
    #[test]
    fn a_failed_root_load_or_reload_tells_nothing() {
        let mut state = ExplorerState::default();

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::LoadFailed {
                message: String::from("no access"),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_node_hover_event_when_reduced_then_hover_path_is_updated() {
        let mut state = ExplorerState::default();

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::NodeHovered {
                path: Some(vec![String::from("src")]),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        let hovered = state.hovered_path().expect("hover path should be set");
        assert_eq!(hovered, &vec![String::from("src")]);
    }

    #[test]
    fn given_root_loaded_when_root_matches_then_tree_is_applied() {
        let mut state = ExplorerState::default();
        let root = PathBuf::from("/tmp");
        state.set_root(Some(root.clone()));

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.tree().len(), 1);
        assert_eq!(state.tree()[0].name(), "main.rs");
    }

    #[test]
    fn given_root_loaded_for_stale_root_when_reduced_then_tree_is_ignored() {
        let mut state = ExplorerState::default();
        state.set_root(Some(PathBuf::from("/tmp/a")));

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::RootLoaded {
                root: PathBuf::from("/tmp/b"),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    PathBuf::from("/tmp/b/main.rs"),
                    false,
                )],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert!(state.tree().is_empty());
    }

    /// Following a new folder clears the tree and asks the binary to read
    /// the new folder.
    #[test]
    fn given_sync_root_with_new_cwd_when_reduced_then_root_is_updated_and_read()
    {
        let root = PathBuf::from("/tmp/original");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/new-root"),
            },
            &ctx(),
        );

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::LoadRoot {
                root: PathBuf::from("/tmp/new-root"),
            }]
        );
        assert_eq!(state.root_label(), Some("new-root"));
        assert!(state.tree().is_empty());
    }

    #[test]
    fn given_sync_root_with_same_cwd_when_reduced_then_tree_is_unchanged() {
        let root = PathBuf::from("/tmp/original");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/original"),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.root_label(), Some("original"));
        assert_eq!(state.tree().len(), 1);
    }

    #[test]
    fn given_watched_directory_changed_when_reduced_then_reload_is_requested() {
        let root = PathBuf::from("/tmp/project");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DirectoryChanged {
                directory: root.clone(),
            },
            &ctx(),
        );

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::ReloadFolder { directory: root }]
        );
    }

    #[test]
    fn given_unknown_directory_changed_when_reduced_then_reload_is_ignored() {
        let root = PathBuf::from("/tmp/project");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DirectoryChanged {
                directory: PathBuf::from("/tmp/other"),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }

    #[test]
    fn given_directory_reload_when_reduced_then_directory_nodes_are_applied() {
        let root = PathBuf::from("/tmp/project");
        let mut state = state_with_src(&root);

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DirectoryReloaded {
                directory: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("Cargo.toml"),
                    root.join("Cargo.toml"),
                    false,
                )],
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.tree().len(), 1);
        assert_eq!(state.tree()[0].name(), "Cargo.toml");
    }

    /// A click on a closed folder that was never read selects it, opens it
    /// and asks the binary to read it.
    #[test]
    fn given_folder_node_when_clicked_then_selection_is_set_and_folder_read() {
        let root = PathBuf::from("/tmp");
        let mut state = state_with_src(&root);

        let outcomes = click(&mut state, "src");

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::LoadFolder {
                path: vec![String::from("src")],
                directory: root.join("src"),
            }]
        );
        assert_eq!(state.selected_path(), Some(&vec![String::from("src")]));
    }

    /// A click on an open folder closes it; nothing has to be read.
    #[test]
    fn given_open_folder_when_clicked_again_then_it_closes_and_reads_nothing() {
        let root = PathBuf::from("/tmp");
        let mut state = state_with_src(&root);
        let _ = click(&mut state, "src");

        let outcomes = click(&mut state, "src");

        assert!(outcomes.is_empty());
        assert!(!state.tree()[0].is_expanded());
    }

    /// A click on a file selects it and asks to open it.
    #[test]
    fn given_file_node_when_clicked_then_it_is_opened() {
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

        let outcomes = click(&mut state, "main.rs");

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::OpenFile {
                file_path: root.join("main.rs"),
            }]
        );
        assert_eq!(state.selected_path(), Some(&vec![String::from("main.rs")]));
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

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::LoadFailed {
                message: String::from("boom"),
            },
            &ctx(),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.tree().len(), 1);
        assert_eq!(state.tree()[0].name(), "main.rs");
    }
}
