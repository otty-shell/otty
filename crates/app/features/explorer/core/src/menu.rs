//! The context menu of the Files tree: it opens on a file, a folder or the
//! empty area, and its items open, name, rename or delete the entry, or
//! open a terminal in a folder.

use crate::ctx::ExplorerCtx;
use crate::disk_operation::DiskOperation;
use crate::event::{ExplorerMenuAction, ExplorerOutcome};
use crate::interaction::{
    ContextMenu, InlineEditKind, MenuTarget, PendingDelete,
};
use crate::naming;
use crate::reducer::open_node;
use crate::state::ExplorerState;
use crate::types::TreePath;

/// Open the menu of the row at `path` at the pointer and select the row.
/// The menu takes the keyboard from the terminal.
pub(crate) fn open_on_node(
    state: &mut ExplorerState,
    path: TreePath,
    ctx: &ExplorerCtx,
) -> Vec<ExplorerOutcome> {
    let Some(node) = state.node(&path) else {
        return Vec::new();
    };
    let target = if node.is_folder() {
        MenuTarget::Folder(path.clone())
    } else {
        MenuTarget::File(path.clone())
    };

    open(state, target, ctx);
    state.set_selected_path(Some(path));

    vec![ExplorerOutcome::TakeKeyboard]
}

/// Open the menu of the followed folder at the pointer; nothing opens while
/// no folder is followed.
pub(crate) fn open_on_background(
    state: &mut ExplorerState,
    ctx: &ExplorerCtx,
) -> Vec<ExplorerOutcome> {
    if state.root().is_none() {
        return Vec::new();
    }

    open(state, MenuTarget::Background, ctx);
    state.set_selected_path(None);

    vec![ExplorerOutcome::TakeKeyboard]
}

/// Close the menu and do what its item `action` asks for the row it was
/// opened on.
pub(crate) fn act(
    state: &mut ExplorerState,
    action: ExplorerMenuAction,
) -> Vec<ExplorerOutcome> {
    let Some(menu) = state.take_context_menu() else {
        return Vec::new();
    };
    let target = menu.target;

    match action {
        ExplorerMenuAction::Open => open_target(state, target),
        ExplorerMenuAction::OpenInTerminal => open_in_terminal(state, &target),
        ExplorerMenuAction::NewFile => match folder_of(target) {
            Some(parent) => {
                naming::start(state, InlineEditKind::NewFile { parent })
            },
            None => Vec::new(),
        },
        ExplorerMenuAction::NewFolder => match folder_of(target) {
            Some(parent) => {
                naming::start(state, InlineEditKind::NewFolder { parent })
            },
            None => Vec::new(),
        },
        ExplorerMenuAction::Rename => match entry_of(target) {
            Some(path) => naming::start(state, InlineEditKind::Rename { path }),
            None => Vec::new(),
        },
        ExplorerMenuAction::Delete => {
            if let Some(path) = entry_of(target) {
                ask_delete(state, &path);
            }

            Vec::new()
        },
    }
}

/// Delete the entry the delete dialog asked about, now that the user
/// confirmed it.
pub(crate) fn confirm_delete(
    state: &mut ExplorerState,
) -> Vec<ExplorerOutcome> {
    let Some(delete) = state.take_pending_delete() else {
        return Vec::new();
    };

    let operation = DiskOperation::Delete { path: delete.path };

    vec![ExplorerOutcome::RunDiskOperation(operation)]
}

fn open(state: &mut ExplorerState, target: MenuTarget, ctx: &ExplorerCtx) {
    state.set_drag(None);
    state.set_pressed_path(None);
    state.set_context_menu(Some(ContextMenu {
        target,
        cursor: ctx.cursor,
        area_size: ctx.screen_size,
    }));
}

/// Open a file in the editor, or open a folder in the tree; an open folder
/// stays open.
fn open_target(
    state: &mut ExplorerState,
    target: MenuTarget,
) -> Vec<ExplorerOutcome> {
    match target {
        MenuTarget::File(path) => open_node(state, path),
        MenuTarget::Folder(path) => {
            state.set_selected_path(Some(path.clone()));

            match state.expand_folder(&path) {
                Some(directory) => {
                    vec![ExplorerOutcome::LoadFolder { path, directory }]
                },
                None => Vec::new(),
            }
        },
        MenuTarget::Background => Vec::new(),
    }
}

fn open_in_terminal(
    state: &ExplorerState,
    target: &MenuTarget,
) -> Vec<ExplorerOutcome> {
    let folder = match target {
        MenuTarget::Background => state.folder_on_disk(&[]),
        MenuTarget::Folder(path) => state.folder_on_disk(path),
        MenuTarget::File(_) => None,
    };

    match folder {
        Some(folder) => vec![ExplorerOutcome::OpenTerminal { folder }],
        None => Vec::new(),
    }
}

/// Ask the user before the entry at `path` is deleted.
fn ask_delete(state: &mut ExplorerState, path: &[String]) {
    let Some(node) = state.node(path) else {
        return;
    };

    let delete = PendingDelete {
        path: node.path().to_path_buf(),
        name: node.name().to_string(),
        is_folder: node.is_folder(),
    };
    state.set_pending_delete(Some(delete));
}

/// The folder a new entry goes in: the followed folder for the empty area,
/// the folder itself for a folder row, none for a file row.
fn folder_of(target: MenuTarget) -> Option<TreePath> {
    match target {
        MenuTarget::Background => Some(TreePath::new()),
        MenuTarget::Folder(path) => Some(path),
        MenuTarget::File(_) => None,
    }
}

/// The entry a rename or a deletion is about: the row, never the followed
/// folder itself.
fn entry_of(target: MenuTarget) -> Option<TreePath> {
    match target {
        MenuTarget::Folder(path) | MenuTarget::File(path) => Some(path),
        MenuTarget::Background => None,
    }
}

#[cfg(test)]
mod tests {
    use otty_app_i18n as i18n;

    use crate::disk_operation::DiskOperation;
    use crate::event::{ExplorerIntent, ExplorerMenuAction, ExplorerOutcome};
    use crate::interaction::{InlineEditKind, MenuTarget, PendingDelete};
    use crate::reducer::reduce;
    use crate::state::ExplorerState;
    use crate::testing::{ROOT, ctx, on_disk, path, state_with_tree};

    fn right_click(state: &mut ExplorerState, names: &[&str]) {
        let _ = reduce(
            state,
            ExplorerIntent::NodeRightClicked { path: path(names) },
            &ctx(),
        );
    }

    fn choose(
        state: &mut ExplorerState,
        action: ExplorerMenuAction,
    ) -> Vec<ExplorerOutcome> {
        reduce(state, ExplorerIntent::ContextMenuAction(action), &ctx())
    }

    /// A right click on a file opens its menu at the pointer, selects the
    /// file and takes the keyboard from the terminal.
    #[test]
    fn a_right_click_on_a_file_opens_the_file_menu() {
        let mut state = state_with_tree();

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::NodeRightClicked {
                path: path(&["main.rs"]),
            },
            &ctx(),
        );

        assert_eq!(outcomes, vec![ExplorerOutcome::TakeKeyboard]);
        let menu = state.context_menu().expect("the menu should open");
        assert_eq!(menu.target, MenuTarget::File(path(&["main.rs"])));
        assert_eq!(menu.cursor, ctx().cursor);
        assert_eq!(state.selected_path(), Some(&path(&["main.rs"])));
    }

    #[test]
    fn a_right_click_on_a_folder_opens_the_folder_menu() {
        let mut state = state_with_tree();

        right_click(&mut state, &["docs"]);

        let menu = state.context_menu().expect("the menu should open");
        assert_eq!(menu.target, MenuTarget::Folder(path(&["docs"])));
    }

    #[test]
    fn a_right_click_on_the_empty_area_opens_the_menu_of_the_followed_folder() {
        let mut state = state_with_tree();

        let outcomes =
            reduce(&mut state, ExplorerIntent::BackgroundRightClicked, &ctx());

        assert_eq!(outcomes, vec![ExplorerOutcome::TakeKeyboard]);
        let menu = state.context_menu().expect("the menu should open");
        assert_eq!(menu.target, MenuTarget::Background);
    }

    /// Without a followed folder there is nothing to act on.
    #[test]
    fn a_right_click_without_a_followed_folder_opens_nothing() {
        let mut state = ExplorerState::default();

        let outcomes =
            reduce(&mut state, ExplorerIntent::BackgroundRightClicked, &ctx());

        assert!(outcomes.is_empty());
        assert!(state.context_menu().is_none());
    }

    #[test]
    fn open_on_a_file_opens_it_in_the_editor_and_gives_the_keys_back() {
        let mut state = state_with_tree();
        right_click(&mut state, &["main.rs"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::Open);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::OpenFile {
                    file_path: on_disk("main.rs"),
                },
                ExplorerOutcome::OverlayClosed,
            ]
        );
        assert!(state.context_menu().is_none());
    }

    /// Open on a closed folder opens it in the tree and reads it.
    #[test]
    fn open_on_a_folder_opens_it_in_the_tree() {
        let mut state = state_with_tree();
        right_click(&mut state, &["docs"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::Open);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::LoadFolder {
                    path: path(&["docs"]),
                    directory: on_disk("docs"),
                },
                ExplorerOutcome::OverlayClosed,
            ]
        );
        let docs = state.node(&path(&["docs"])).expect("docs");
        assert!(docs.is_expanded());
    }

    /// Open on a folder that is open already leaves it open.
    #[test]
    fn open_on_an_open_folder_keeps_it_open() {
        let mut state = state_with_tree();
        right_click(&mut state, &["src"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::Open);

        assert_eq!(outcomes, vec![ExplorerOutcome::OverlayClosed]);
        assert!(state.node(&path(&["src"])).expect("src").is_expanded());
    }

    #[test]
    fn open_in_terminal_on_a_folder_opens_a_terminal_there() {
        let mut state = state_with_tree();
        right_click(&mut state, &["docs"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::OpenInTerminal);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::OpenTerminal {
                    folder: on_disk("docs"),
                },
                ExplorerOutcome::OverlayClosed,
            ]
        );
    }

    #[test]
    fn open_in_terminal_on_the_empty_area_opens_the_followed_folder() {
        let mut state = state_with_tree();
        let _ =
            reduce(&mut state, ExplorerIntent::BackgroundRightClicked, &ctx());

        let outcomes = choose(&mut state, ExplorerMenuAction::OpenInTerminal);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::OpenTerminal {
                    folder: ROOT.into(),
                },
                ExplorerOutcome::OverlayClosed,
            ]
        );
    }

    /// New file on a closed folder opens and reads the folder, and the name
    /// field takes the keyboard in place of the menu.
    #[test]
    fn new_file_on_a_closed_folder_opens_it_and_asks_for_the_name() {
        let mut state = state_with_tree();
        right_click(&mut state, &["docs"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::NewFile);

        let edit = state.inline_edit().expect("the name field should show");
        assert_eq!(
            edit.kind,
            InlineEditKind::NewFile {
                parent: path(&["docs"]),
            }
        );
        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::LoadFolder {
                    path: path(&["docs"]),
                    directory: on_disk("docs"),
                },
                ExplorerOutcome::FocusInput(edit.id.clone()),
            ]
        );
        assert!(state.context_menu().is_none());
    }

    #[test]
    fn new_folder_on_the_empty_area_asks_for_a_name_in_the_followed_folder() {
        let mut state = state_with_tree();
        let _ =
            reduce(&mut state, ExplorerIntent::BackgroundRightClicked, &ctx());

        let outcomes = choose(&mut state, ExplorerMenuAction::NewFolder);

        let edit = state.inline_edit().expect("the name field should show");
        assert_eq!(edit.kind, InlineEditKind::NewFolder { parent: path(&[]) });
        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::FocusInput(edit.id.clone())]
        );
    }

    #[test]
    fn rename_asks_for_a_new_name_starting_from_the_old_one() {
        let mut state = state_with_tree();
        right_click(&mut state, &["src", "lib.rs"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::Rename);

        let edit = state.inline_edit().expect("the name field should show");
        assert_eq!(
            edit.kind,
            InlineEditKind::Rename {
                path: path(&["src", "lib.rs"]),
            }
        );
        assert_eq!(edit.value, "lib.rs");
        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::FocusInput(edit.id.clone())]
        );
    }

    /// Delete asks first: the dialog takes the place of the menu, and
    /// nothing is deleted yet.
    #[test]
    fn delete_asks_before_it_deletes() {
        let mut state = state_with_tree();
        right_click(&mut state, &["src"]);

        let outcomes = choose(&mut state, ExplorerMenuAction::Delete);

        assert!(outcomes.is_empty());
        assert_eq!(
            state.pending_delete(),
            Some(&PendingDelete {
                path: on_disk("src"),
                name: String::from("src"),
                is_folder: true,
            })
        );
    }

    #[test]
    fn a_confirmed_delete_deletes_the_entry() {
        let mut state = state_with_tree();
        right_click(&mut state, &["main.rs"]);
        let _ = choose(&mut state, ExplorerMenuAction::Delete);

        let outcomes =
            reduce(&mut state, ExplorerIntent::DeleteConfirmed, &ctx());

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::RunDiskOperation(DiskOperation::Delete {
                    path: on_disk("main.rs"),
                }),
                ExplorerOutcome::OverlayClosed,
            ]
        );
        assert!(state.pending_delete().is_none());
    }

    #[test]
    fn a_cancelled_delete_deletes_nothing() {
        let mut state = state_with_tree();
        right_click(&mut state, &["main.rs"]);
        let _ = choose(&mut state, ExplorerMenuAction::Delete);

        let outcomes =
            reduce(&mut state, ExplorerIntent::DeleteCancelled, &ctx());

        assert_eq!(outcomes, vec![ExplorerOutcome::OverlayClosed]);
        assert!(state.pending_delete().is_none());
    }

    #[test]
    fn a_dismissed_menu_closes_and_gives_the_keys_back() {
        let mut state = state_with_tree();
        right_click(&mut state, &["main.rs"]);

        let outcomes =
            reduce(&mut state, ExplorerIntent::ContextMenuDismiss, &ctx());

        assert_eq!(outcomes, vec![ExplorerOutcome::OverlayClosed]);
        assert!(state.context_menu().is_none());
    }

    /// A failed disk operation tells the user which entry failed and why.
    #[test]
    fn a_failed_change_tells_the_user() {
        let mut state = state_with_tree();
        let operation = DiskOperation::Delete {
            path: on_disk("main.rs"),
        };

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DiskOperationFailed {
                operation,
                message: String::from("no access"),
            },
            &ctx(),
        );

        let text = i18n::files_change_failed("main.rs", "no access");
        assert_eq!(outcomes, vec![ExplorerOutcome::Failed(text)]);
    }

    /// A done disk operation reads again the folders it changed that the tree
    /// shows; the watch may be late or miss it.
    #[test]
    fn a_made_change_reads_the_changed_folders_again() {
        let mut state = state_with_tree();
        let operation = DiskOperation::Move {
            from: on_disk("main.rs"),
            to: on_disk("src/main.rs"),
        };

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DiskOperationDone { operation },
            &ctx(),
        );

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::ReloadFolder {
                    directory: ROOT.into(),
                },
                ExplorerOutcome::ReloadFolder {
                    directory: on_disk("src"),
                },
            ]
        );
    }

    /// A disk operation in a folder the tree does not show reads nothing.
    #[test]
    fn a_change_in_an_unread_folder_reads_nothing() {
        let mut state = state_with_tree();
        let operation = DiskOperation::CreateFile {
            path: on_disk("docs/notes.md"),
        };

        let outcomes = reduce(
            &mut state,
            ExplorerIntent::DiskOperationDone { operation },
            &ctx(),
        );

        assert!(outcomes.is_empty());
    }
}
