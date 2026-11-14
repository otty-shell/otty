//! Dragging a row of the Files tree onto a folder moves the entry there.
//! A press holds the row; once the pointer moves far enough it becomes a
//! drag, and the release drops it on the folder under the pointer.

use otty_app_i18n::{self as i18n, Key};

use crate::disk_operation::DiskOperation;
use crate::event::ExplorerOutcome;
use crate::interaction::{Drag, DropTarget};
use crate::naming;
use crate::state::ExplorerState;
use crate::types::TreePath;

/// How far the pointer moves from the press, in pixels, before a held row
/// becomes a drag. A shorter move is still a click.
const DRAG_DISTANCE: f32 = 4.0;

/// Select the pressed row and hold it: it may become a drag.
pub(crate) fn hold(state: &mut ExplorerState, path: TreePath) {
    state.set_pressed_path(Some(path.clone()));
    state.set_selected_path(Some(path.clone()));
    state.set_drag(Some(Drag {
        source: path,
        origin: state.cursor(),
        is_active: false,
    }));
}

/// The pointer moved: a held row far enough from its press becomes a drag,
/// and a drag finds where it would land.
pub(crate) fn follow_cursor(state: &mut ExplorerState) {
    let cursor = state.cursor();
    let Some(drag) = state.drag_mut() else {
        return;
    };

    let dx = cursor.x - drag.origin.x;
    let dy = cursor.y - drag.origin.y;
    if dx * dx + dy * dy >= DRAG_DISTANCE * DRAG_DISTANCE {
        drag.is_active = true;
    }

    follow_hover(state);
}

/// The hovered row changed: a drag finds where it would land.
pub(crate) fn follow_hover(state: &mut ExplorerState) {
    let Some(source) = active_source(state) else {
        return;
    };

    let target = target_under_pointer(state);
    let allowed = allowed(state, &source, target);
    state.set_drop_target(allowed);
}

/// A release on the empty area drops a dragged row into the followed
/// folder; otherwise it lets the held row go.
pub(crate) fn release_on_background(
    state: &mut ExplorerState,
) -> Vec<ExplorerOutcome> {
    if let Some(source) = active_source(state) {
        let allowed = allowed(state, &source, DropTarget::Root);
        state.set_drop_target(allowed);
    }

    if let Some(dropped) = finish(state) {
        return dropped;
    }

    state.set_pressed_path(None);
    Vec::new()
}

/// Let the held row go. A drag drops the row where it would land and
/// returns what that asks for, empty where it may not land; a held row that
/// never became a drag returns `None`, so the release may be a click.
pub(crate) fn finish(
    state: &mut ExplorerState,
) -> Option<Vec<ExplorerOutcome>> {
    let drag = state.take_drag()?;
    let target = state.drop_target().cloned();
    state.set_drop_target(None);
    if !drag.is_active {
        return None;
    }

    state.set_pressed_path(None);

    let Some(target) = target else {
        return Some(Vec::new());
    };
    Some(drop(state, &drag.source, &target))
}

/// The dragged row, once the held row became a drag.
fn active_source(state: &ExplorerState) -> Option<TreePath> {
    state
        .drag()
        .filter(|drag| drag.is_active)
        .map(|drag| drag.source.clone())
}

/// Where a row dropped now would land: on the hovered folder, in the folder
/// of the hovered file, or in the followed folder away from the rows.
fn target_under_pointer(state: &ExplorerState) -> DropTarget {
    let Some(hovered) = state.hovered_path() else {
        return DropTarget::Root;
    };
    let Some(node) = state.node(hovered) else {
        return DropTarget::Root;
    };

    if node.is_folder() {
        return DropTarget::Folder(hovered.clone());
    }

    match hovered.split_last() {
        Some((_, parent)) if !parent.is_empty() => {
            DropTarget::Folder(parent.to_vec())
        },
        _ => DropTarget::Root,
    }
}

/// `target`, if the row at `source` may land there: not in the folder it is
/// in already, and a folder not in itself or a folder inside it.
fn allowed(
    state: &ExplorerState,
    source: &[String],
    target: DropTarget,
) -> Option<DropTarget> {
    let node = state.node(source)?;
    let (_, source_folder) = source.split_last()?;
    let target_folder = target.folder_path();

    if source_folder == target_folder {
        return None;
    }
    if node.is_folder() && target_folder.starts_with(source) {
        return None;
    }

    Some(target)
}

/// Move the row at `source` into the folder of `target`, unless the folder
/// already has an entry with its name.
fn drop(
    state: &ExplorerState,
    source: &[String],
    target: &DropTarget,
) -> Vec<ExplorerOutcome> {
    let Some(node) = state.node(source) else {
        return Vec::new();
    };
    let name = node.name();

    if naming::is_taken(state, target.folder_path(), name) {
        let reason = i18n::t(Key::FilesNameTaken);
        let text = i18n::files_change_failed(name, reason);
        return vec![ExplorerOutcome::Failed(text)];
    }

    let Some(folder) = state.folder_on_disk(target.folder_path()) else {
        return Vec::new();
    };
    let operation = DiskOperation::Move {
        from: node.path().to_path_buf(),
        to: folder.join(name),
    };

    vec![ExplorerOutcome::RunDiskOperation(operation)]
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use otty_app_i18n::{self as i18n, Key};

    use crate::disk_operation::DiskOperation;
    use crate::event::{ExplorerIntent, ExplorerOutcome};
    use crate::interaction::DropTarget;
    use crate::reducer::reduce;
    use crate::state::ExplorerState;
    use crate::testing::{ctx, on_disk, path, state_with_tree};
    use crate::types::FileNode;

    fn send(
        state: &mut ExplorerState,
        intent: ExplorerIntent,
    ) -> Vec<ExplorerOutcome> {
        reduce(state, intent, &ctx())
    }

    fn move_to(state: &mut ExplorerState, x: f32, y: f32) {
        let _ = send(
            state,
            ExplorerIntent::CursorMoved {
                position: Point::new(x, y),
            },
        );
    }

    fn hover(state: &mut ExplorerState, names: &[&str]) {
        let _ = send(
            state,
            ExplorerIntent::NodeHovered {
                path: Some(path(names)),
            },
        );
    }

    /// Press the row at `names` and drag it far enough to start a drag.
    fn drag(state: &mut ExplorerState, names: &[&str]) {
        move_to(state, 10.0, 10.0);
        hover(state, names);
        let _ = send(state, ExplorerIntent::NodePressed { path: path(names) });
        move_to(state, 10.0, 40.0);
    }

    #[test]
    fn a_file_dropped_on_a_folder_moves_into_it() {
        let mut state = state_with_tree();
        drag(&mut state, &["main.rs"]);
        hover(&mut state, &["src"]);

        let outcomes = send(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: path(&["src"]),
            },
        );

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::RunDiskOperation(DiskOperation::Move {
                from: on_disk("main.rs"),
                to: on_disk("src/main.rs"),
            })]
        );
        assert!(state.drag().is_none());
        assert!(state.drop_target().is_none());
    }

    /// Over a file, the drop lands in the folder of that file.
    #[test]
    fn a_row_dragged_over_a_file_lands_in_its_folder() {
        let mut state = state_with_tree();
        drag(&mut state, &["main.rs"]);

        hover(&mut state, &["src", "lib.rs"]);

        assert_eq!(
            state.drop_target(),
            Some(&DropTarget::Folder(path(&["src"])))
        );
    }

    #[test]
    fn a_row_dropped_on_the_empty_area_moves_to_the_followed_folder() {
        let mut state = state_with_tree();
        drag(&mut state, &["src", "lib.rs"]);

        let outcomes = send(&mut state, ExplorerIntent::BackgroundReleased);

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::RunDiskOperation(DiskOperation::Move {
                from: on_disk("src/lib.rs"),
                to: on_disk("lib.rs"),
            })]
        );
    }

    /// A press and a release that moved less than the drag distance is a
    /// click: the file opens.
    #[test]
    fn a_small_move_between_press_and_release_is_a_click() {
        let mut state = state_with_tree();
        move_to(&mut state, 10.0, 10.0);
        let _ = send(
            &mut state,
            ExplorerIntent::NodePressed {
                path: path(&["main.rs"]),
            },
        );
        move_to(&mut state, 11.0, 12.0);

        let outcomes = send(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: path(&["main.rs"]),
            },
        );

        assert_eq!(
            outcomes,
            vec![ExplorerOutcome::OpenFile {
                file_path: on_disk("main.rs"),
            }]
        );
    }

    /// A folder can not go into itself or a folder inside it.
    #[test]
    fn a_folder_dropped_into_itself_moves_nowhere() {
        let mut state = state_with_tree();
        drag(&mut state, &["src"]);
        hover(&mut state, &["src", "lib.rs"]);

        assert_eq!(state.drop_target(), None);

        let outcomes = send(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: path(&["src", "lib.rs"]),
            },
        );

        assert!(outcomes.is_empty());
    }

    /// Dropping a row where it already is changes nothing.
    #[test]
    fn a_row_dropped_in_its_own_folder_moves_nowhere() {
        let mut state = state_with_tree();
        drag(&mut state, &["src", "lib.rs"]);
        hover(&mut state, &["src"]);

        assert_eq!(state.drop_target(), None);

        let outcomes = send(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: path(&["src"]),
            },
        );

        assert!(outcomes.is_empty());
    }

    /// A drop on a name the folder already has tells the user instead of
    /// replacing the entry there.
    #[test]
    fn a_drop_on_a_taken_name_tells_the_user() {
        let mut state = state_with_tree();
        let _ = state.apply_folder_nodes(
            &path(&["src"]),
            vec![FileNode::new(
                String::from("main.rs"),
                on_disk("src/main.rs"),
                false,
            )],
        );
        drag(&mut state, &["main.rs"]);
        hover(&mut state, &["src"]);

        let outcomes = send(
            &mut state,
            ExplorerIntent::NodeReleased {
                path: path(&["src"]),
            },
        );

        let text =
            i18n::files_change_failed("main.rs", i18n::t(Key::FilesNameTaken));
        assert_eq!(outcomes, vec![ExplorerOutcome::Failed(text)]);
    }
}
