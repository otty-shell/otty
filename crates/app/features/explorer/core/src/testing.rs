//! A small followed folder the tests of the explorer start from.

use std::path::PathBuf;

use iced::{Point, Size};

use crate::ctx::ExplorerCtx;
use crate::state::ExplorerState;
use crate::types::{FileNode, TreePath};

/// The followed folder of [`state_with_tree`].
pub(crate) const ROOT: &str = "/p";

/// The pointer and the main area the tests reduce with.
pub(crate) fn ctx() -> ExplorerCtx {
    ExplorerCtx {
        cursor: Point::new(40.0, 50.0),
        screen_size: Size::new(1024.0, 768.0),
    }
}

/// A tree place from its names.
pub(crate) fn path(names: &[&str]) -> TreePath {
    names.iter().map(|name| String::from(*name)).collect()
}

/// The disk path of `relative` in [`ROOT`].
pub(crate) fn on_disk(relative: &str) -> PathBuf {
    PathBuf::from(ROOT).join(relative)
}

/// A state that follows [`ROOT`] and shows:
///
/// - `src`, an open folder that was read, with the file `lib.rs`;
/// - `docs`, a closed folder that was never read;
/// - `main.rs`, a file.
pub(crate) fn state_with_tree() -> ExplorerState {
    let root = PathBuf::from(ROOT);
    let mut state = ExplorerState::default();
    state.set_root(Some(root.clone()));
    let _ = state.apply_root_nodes(
        &root,
        vec![
            FileNode::new(String::from("docs"), on_disk("docs"), true),
            FileNode::new(String::from("src"), on_disk("src"), true),
            FileNode::new(String::from("main.rs"), on_disk("main.rs"), false),
        ],
    );
    let _ = state.toggle_folder(&path(&["src"]));
    let _ = state.apply_folder_nodes(
        &path(&["src"]),
        vec![FileNode::new(
            String::from("lib.rs"),
            on_disk("src/lib.rs"),
            false,
        )],
    );

    state
}
