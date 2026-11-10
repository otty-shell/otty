//! The explorer feature of the OTTY app: the Files section of the right
//! panel in Terminal mode. It shows the folder the active terminal is in as
//! a tree, opens folders in place and asks to open a pressed file.
//!
//! The feature reads no folder and watches nothing itself. It reports what
//! it needs as an [`ExplorerOutcome`]: the binary reads the folder and gives
//! its entries back as an [`ExplorerIntent`]. Which folders to watch for
//! changes is decided here, by [`Explorer::watched_folders`]; the binary runs
//! the watch and sends [`ExplorerIntent::DirectoryChanged`].
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//!
//! use otty_app_explorer::{
//!     Explorer, ExplorerIntent, ExplorerOutcome, FileNode,
//! };
//! use otty_app_slots::Feature;
//!
//! let mut explorer = Explorer::new();
//! let root = PathBuf::from("/home/user/project");
//!
//! let follow =
//!     explorer.reduce(ExplorerIntent::SyncRoot { cwd: root.clone() }, ());
//! assert_eq!(
//!     follow,
//!     vec![ExplorerOutcome::LoadRoot { root: root.clone() }]
//! );
//!
//! // The binary read the folder and gives its entries back.
//! let src = FileNode::new(String::from("src"), root.join("src"), true);
//! let _ = explorer.reduce(
//!     ExplorerIntent::RootLoaded {
//!         root: root.clone(),
//!         nodes: vec![src],
//!     },
//!     (),
//! );
//!
//! let press = explorer.reduce(
//!     ExplorerIntent::NodePressed {
//!         path: vec![String::from("src")],
//!     },
//!     (),
//! );
//! assert_eq!(
//!     press,
//!     vec![ExplorerOutcome::LoadFolder {
//!         path: vec![String::from("src")],
//!         directory: root.join("src"),
//!     }]
//! );
//! assert_eq!(
//!     explorer.watched_folders(),
//!     vec![root.clone(), root.join("src")]
//! );
//! ```

mod event;
mod model;
mod reducer;
mod state;
mod types;
mod view;

use std::path::PathBuf;

pub use event::{ExplorerIntent, ExplorerOutcome};
use iced::Element;
use model::SidebarTreeViewModel;
use otty_app_slots::{Feature, RightPanel};
use otty_app_theme::theme::ThemeProps;
use state::ExplorerState;
pub use types::{FileNode, TreePath};
use view::sidebar_tree;

/// The explorer feature: the tree of the followed folder, with the selected
/// and hovered rows.
///
/// # Example
///
/// ```
/// use otty_app_explorer::Explorer;
///
/// let explorer = Explorer::new();
///
/// assert!(explorer.watched_folders().is_empty());
/// ```
pub struct Explorer {
    state: ExplorerState,
}

impl Explorer {
    /// Return the folders to watch for changes on disk, sorted: the
    /// followed folder and every folder of the tree that was read. Empty
    /// while no folder is followed.
    ///
    /// The binary runs the watch: it needs a file watcher and an async
    /// runtime, which a feature crate does not take. Each change comes back
    /// as [`ExplorerIntent::DirectoryChanged`].
    pub fn watched_folders(&self) -> Vec<PathBuf> {
        self.state.watched_directories()
    }

    /// Create the feature with no folder to follow.
    pub fn new() -> Self {
        Self {
            state: ExplorerState::default(),
        }
    }
}

impl Default for Explorer {
    fn default() -> Self {
        Self::new()
    }
}

impl Feature for Explorer {
    type Intent = ExplorerIntent;
    type Outcome = ExplorerOutcome;
    type Ctx<'a> = ();

    fn reduce(
        &mut self,
        intent: ExplorerIntent,
        _ctx: (),
    ) -> Vec<ExplorerOutcome> {
        reducer::reduce(&mut self.state, intent)
    }
}

impl RightPanel for Explorer {
    /// The Files section: its header, the line with the followed folder and
    /// the tree.
    fn right_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, ExplorerIntent> {
        sidebar_tree::view(sidebar_tree::SidebarTreeProps {
            vm: SidebarTreeViewModel::from_state(&self.state),
            theme,
        })
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    #[test]
    fn given_empty_explorer_when_watched_folders_requested_then_none_are_returned()
     {
        let explorer = Explorer::new();

        assert!(explorer.watched_folders().is_empty());
    }

    #[test]
    fn given_rooted_explorer_when_watched_folders_requested_then_root_is_returned()
     {
        let mut explorer = Explorer::new();

        let _ = explorer.reduce(
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/project"),
            },
            (),
        );

        assert_eq!(
            explorer.watched_folders(),
            vec![PathBuf::from("/tmp/project")]
        );
    }

    #[test]
    fn the_right_panel_shows_the_files_section_with_the_followed_folder() {
        let themes = ThemeManager::new();
        let mut explorer = Explorer::new();
        let _ = explorer.reduce(
            ExplorerIntent::SyncRoot {
                cwd: PathBuf::from("/tmp/project"),
            },
            (),
        );

        let mut ui =
            simulator(explorer.right_panel(ThemeProps::new(themes.current())));

        assert!(ui.find("Files").is_ok());
        assert!(ui.find("project").is_ok());
    }
}
