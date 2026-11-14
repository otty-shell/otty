//! The name field of the Files tree: a name for a new file or folder, or a
//! new name of an entry, typed in place of a row and checked before the
//! disk is asked for the operation.

use std::path::MAIN_SEPARATOR;

use otty_app_i18n::{self as i18n, Key};

use crate::disk_operation::DiskOperation;
use crate::event::ExplorerOutcome;
use crate::interaction::{InlineEdit, InlineEditKind};
use crate::state::ExplorerState;

/// Show the name field for `kind` and give it the keyboard. A new entry
/// opens its folder first, and reads it when it was never read; a rename
/// starts from the old name.
pub(crate) fn start(
    state: &mut ExplorerState,
    kind: InlineEditKind,
) -> Vec<ExplorerOutcome> {
    let mut outcomes = Vec::new();

    let value = match &kind {
        InlineEditKind::NewFile { parent }
        | InlineEditKind::NewFolder { parent } => {
            if let Some(directory) = state.expand_folder(parent) {
                outcomes.push(ExplorerOutcome::LoadFolder {
                    path: parent.clone(),
                    directory,
                });
            }

            String::new()
        },
        InlineEditKind::Rename { path } => match state.node(path) {
            Some(node) => node.name().to_string(),
            None => return Vec::new(),
        },
    };

    let edit = InlineEdit::new(kind, value);
    outcomes.push(ExplorerOutcome::FocusInput(edit.id.clone()));
    state.set_inline_edit(Some(edit));

    outcomes
}

/// Take the typed text; the error of the last try goes.
pub(crate) fn update_value(state: &mut ExplorerState, value: String) {
    let Some(edit) = state.inline_edit_mut() else {
        return;
    };

    edit.value = value;
    edit.error = None;
}

/// Use the typed name: ask the disk for the new entry or the rename. A name
/// that can not be used keeps the field open with the reason; the old name
/// of a rename closes it with nothing to do.
pub(crate) fn submit(state: &mut ExplorerState) -> Vec<ExplorerOutcome> {
    let Some(mut edit) = state.take_inline_edit() else {
        return Vec::new();
    };
    let name = edit.value.trim().to_string();

    match check(state, &edit.kind, &name) {
        Ok(Some(operation)) => {
            vec![ExplorerOutcome::RunDiskOperation(operation)]
        },
        Ok(None) => Vec::new(),
        Err(reason) => {
            edit.error = Some(i18n::t(reason).to_string());
            state.set_inline_edit(Some(edit));

            Vec::new()
        },
    }
}

/// Return whether an entry of the folder at `folder` of the tree already
/// has `name`, as far as the tree knows it; a folder that was not read has
/// no names.
pub(crate) fn is_taken(
    state: &ExplorerState,
    folder: &[String],
    name: &str,
) -> bool {
    state.names_in_folder(folder).contains(&name)
}

/// Check `name` for `kind` and build the disk operation it asks for.
/// `Ok(None)` when there is nothing to do: a rename to the same name, or an
/// entry that is gone from the tree.
fn check(
    state: &ExplorerState,
    kind: &InlineEditKind,
    name: &str,
) -> Result<Option<DiskOperation>, Key> {
    if name.is_empty() {
        return Err(Key::FilesNameEmpty);
    }
    if !is_single_path_part(name) {
        return Err(Key::FilesNameInvalid);
    }

    match kind {
        InlineEditKind::NewFile { parent } => {
            if is_taken(state, parent, name) {
                return Err(Key::FilesNameTaken);
            }

            let operation = state.folder_on_disk(parent).map(|folder| {
                DiskOperation::CreateFile {
                    path: folder.join(name),
                }
            });
            Ok(operation)
        },
        InlineEditKind::NewFolder { parent } => {
            if is_taken(state, parent, name) {
                return Err(Key::FilesNameTaken);
            }

            let operation = state.folder_on_disk(parent).map(|folder| {
                DiskOperation::CreateFolder {
                    path: folder.join(name),
                }
            });
            Ok(operation)
        },
        InlineEditKind::Rename { path } => {
            let Some((old_name, parent)) = path.split_last() else {
                return Ok(None);
            };
            if old_name == name {
                return Ok(None);
            }
            if is_taken(state, parent, name) {
                return Err(Key::FilesNameTaken);
            }

            let operation = state.node_path(path).and_then(|from| {
                let to = from.parent()?.join(name);
                Some(DiskOperation::Move { from, to })
            });
            Ok(operation)
        },
    }
}

/// Whether `name` names one entry in a folder: no path separator, and not
/// the folder itself (`.`) or its parent (`..`).
fn is_single_path_part(name: &str) -> bool {
    let has_separator = name.contains('/') || name.contains(MAIN_SEPARATOR);

    !has_separator && name != "." && name != ".."
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};

    use crate::disk_operation::DiskOperation;
    use crate::event::{ExplorerIntent, ExplorerMenuAction, ExplorerOutcome};
    use crate::reducer::reduce;
    use crate::state::ExplorerState;
    use crate::testing::{ctx, on_disk, path, state_with_tree};

    /// A state with the name field open for `action` on the row at `names`,
    /// or on the empty area when `names` is empty, with `typed` in it.
    fn naming(
        names: &[&str],
        action: ExplorerMenuAction,
        typed: &str,
    ) -> ExplorerState {
        let mut state = state_with_tree();
        let open = if names.is_empty() {
            ExplorerIntent::BackgroundRightClicked
        } else {
            ExplorerIntent::NodeRightClicked { path: path(names) }
        };
        let _ = reduce(&mut state, open, &ctx());
        let _ = reduce(
            &mut state,
            ExplorerIntent::ContextMenuAction(action),
            &ctx(),
        );
        let _ = reduce(
            &mut state,
            ExplorerIntent::InlineEditChanged(String::from(typed)),
            &ctx(),
        );

        state
    }

    fn submit(state: &mut ExplorerState) -> Vec<ExplorerOutcome> {
        reduce(state, ExplorerIntent::InlineEditSubmit, &ctx())
    }

    fn error(state: &ExplorerState) -> Option<&str> {
        state.inline_edit().and_then(|edit| edit.error.as_deref())
    }

    #[test]
    fn a_named_new_file_is_created_in_the_followed_folder() {
        let mut state = naming(&[], ExplorerMenuAction::NewFile, "notes.md");

        let outcomes = submit(&mut state);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::RunDiskOperation(DiskOperation::CreateFile {
                    path: on_disk("notes.md"),
                }),
                ExplorerOutcome::OverlayClosed,
            ]
        );
        assert!(state.inline_edit().is_none());
    }

    #[test]
    fn a_named_new_folder_is_created_in_the_chosen_folder() {
        let mut state =
            naming(&["src"], ExplorerMenuAction::NewFolder, "utils");

        let outcomes = submit(&mut state);

        assert_eq!(
            outcomes[0],
            ExplorerOutcome::RunDiskOperation(DiskOperation::CreateFolder {
                path: on_disk("src/utils"),
            })
        );
    }

    /// Spaces around the typed name are dropped.
    #[test]
    fn the_typed_name_is_trimmed() {
        let mut state = naming(&[], ExplorerMenuAction::NewFile, "  a.txt ");

        let outcomes = submit(&mut state);

        assert_eq!(
            outcomes[0],
            ExplorerOutcome::RunDiskOperation(DiskOperation::CreateFile {
                path: on_disk("a.txt"),
            })
        );
    }

    #[test]
    fn a_blank_name_is_refused_and_the_field_stays() {
        let mut state = naming(&[], ExplorerMenuAction::NewFile, "   ");

        let outcomes = submit(&mut state);

        assert!(outcomes.is_empty());
        assert_eq!(error(&state), Some(i18n::t(Key::FilesNameEmpty)));
    }

    /// A name must be one part of a path: no slash, not `.` or `..`.
    #[test]
    fn a_name_that_is_no_single_path_part_is_refused() {
        for typed in ["a/b", "..", "."] {
            let mut state = naming(&[], ExplorerMenuAction::NewFolder, typed);

            let outcomes = submit(&mut state);

            assert!(outcomes.is_empty(), "{typed}");
            assert_eq!(
                error(&state),
                Some(i18n::t(Key::FilesNameInvalid)),
                "{typed}"
            );
        }
    }

    #[test]
    fn a_name_taken_in_the_folder_is_refused() {
        let mut state = naming(&["src"], ExplorerMenuAction::NewFile, "lib.rs");

        let outcomes = submit(&mut state);

        assert!(outcomes.is_empty());
        assert_eq!(error(&state), Some(i18n::t(Key::FilesNameTaken)));
    }

    /// Typing again clears the error of the last try.
    #[test]
    fn typing_clears_the_error() {
        let mut state = naming(&[], ExplorerMenuAction::NewFile, "");
        let _ = submit(&mut state);

        let _ = reduce(
            &mut state,
            ExplorerIntent::InlineEditChanged(String::from("a")),
            &ctx(),
        );

        assert_eq!(error(&state), None);
    }

    #[test]
    fn a_renamed_entry_moves_to_its_new_name_in_its_folder() {
        let mut state =
            naming(&["src", "lib.rs"], ExplorerMenuAction::Rename, "mod.rs");

        let outcomes = submit(&mut state);

        assert_eq!(
            outcomes,
            vec![
                ExplorerOutcome::RunDiskOperation(DiskOperation::Move {
                    from: on_disk("src/lib.rs"),
                    to: on_disk("src/mod.rs"),
                }),
                ExplorerOutcome::OverlayClosed,
            ]
        );
    }

    /// Keeping the old name does nothing: the field closes.
    #[test]
    fn a_rename_to_the_same_name_changes_nothing() {
        let mut state =
            naming(&["main.rs"], ExplorerMenuAction::Rename, "main.rs");

        let outcomes = submit(&mut state);

        assert_eq!(outcomes, vec![ExplorerOutcome::OverlayClosed]);
        assert!(state.inline_edit().is_none());
    }

    #[test]
    fn a_cancelled_name_creates_nothing() {
        let mut state = naming(&[], ExplorerMenuAction::NewFile, "a.txt");

        let outcomes =
            reduce(&mut state, ExplorerIntent::CancelInlineEdit, &ctx());

        assert_eq!(outcomes, vec![ExplorerOutcome::OverlayClosed]);
        assert!(state.inline_edit().is_none());
    }
}
