//! The explorer feature of the OTTY app: the Files section of the right
//! panel in Terminal mode. It shows the folder the active terminal is in as
//! a tree, opens folders in place and asks to open a clicked file.
//!
//! A right click opens a context menu that depends on the row: a file opens,
//! is renamed or deleted; a folder also takes a new file or folder and opens
//! a terminal in it; the empty area under the rows acts on the followed
//! folder. A row dragged onto a folder moves there. A deletion asks first,
//! in a dialog.
//!
//! The feature reads no folder, changes no file and watches nothing itself.
//! It reports what it needs as an [`ExplorerOutcome`]: `otty-app-explorer-io`
//! reads the folder or makes the [`DiskOperation`] and gives the answer back as
//! an [`ExplorerIntent`]. Which folders to watch for changes is decided here,
//! by [`Explorer::watched_folders`]; `otty-app-explorer-io` runs the watch
//! and sends [`ExplorerIntent::DirectoryChanged`].
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//!
//! use iced::{Point, Size};
//! use otty_app_explorer::{
//!     Explorer, ExplorerCtx, ExplorerIntent, ExplorerOutcome, FileNode,
//! };
//! use otty_app_slots::Component;
//!
//! let ctx = ExplorerCtx {
//!     cursor: Point::ORIGIN,
//!     screen_size: Size::new(800.0, 600.0),
//! };
//! let mut explorer = Explorer::new();
//! let root = PathBuf::from("/home/user/project");
//!
//! let follow =
//!     explorer.reduce(ExplorerIntent::SyncRoot { cwd: root.clone() }, ctx);
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
//!     ctx,
//! );
//!
//! // A click is a press and a release on the same row.
//! let path = vec![String::from("src")];
//! let _ = explorer
//!     .reduce(ExplorerIntent::NodePressed { path: path.clone() }, ctx);
//! let click = explorer
//!     .reduce(ExplorerIntent::NodeReleased { path: path.clone() }, ctx);
//! assert_eq!(
//!     click,
//!     vec![ExplorerOutcome::LoadFolder {
//!         path,
//!         directory: root.join("src"),
//!     }]
//! );
//! assert_eq!(
//!     explorer.watched_folders(),
//!     vec![root.clone(), root.join("src")]
//! );
//! ```

mod ctx;
mod disk_operation;
mod drag;
mod event;
mod interaction;
mod menu;
mod model;
mod naming;
mod reducer;
mod state;
#[cfg(test)]
mod testing;
mod types;
mod view;

use std::path::PathBuf;

pub use ctx::ExplorerCtx;
pub use disk_operation::DiskOperation;
pub use event::{ExplorerIntent, ExplorerMenuAction, ExplorerOutcome};
use iced::Element;
use model::{
    ContextMenuViewModel, DeleteDialogViewModel, SidebarTreeViewModel,
};
use otty_app_slots::{Component, Overlay, Overlays, RightPanel};
use otty_app_theme::theme::ThemeProps;
use state::ExplorerState;
pub use types::{FileNode, TreePath};
use view::{context_menu, delete_dialog, sidebar_tree};

/// The explorer feature: the tree of the followed folder, with the selected
/// and hovered rows, its context menu, name field, delete dialog and drag.
///
/// # Example
///
/// ```
/// use otty_app_explorer::Explorer;
///
/// let explorer = Explorer::new();
///
/// assert!(explorer.watched_folders().is_empty());
/// assert!(!explorer.has_context_menu());
/// ```
pub struct Explorer {
    state: ExplorerState,
}

impl Explorer {
    /// Return the folders to watch for changes on disk, sorted: the
    /// followed folder and every folder of the tree that was read. Empty
    /// while no folder is followed.
    ///
    /// `otty-app-explorer-io` runs the watch: it needs a file watcher,
    /// which a feature crate does not take. Each change comes back as
    /// [`ExplorerIntent::DirectoryChanged`].
    pub fn watched_folders(&self) -> Vec<PathBuf> {
        self.state.watched_directories()
    }

    /// Return whether the context menu of Files is open.
    pub fn has_context_menu(&self) -> bool {
        self.state.context_menu().is_some()
    }

    /// Return whether a name is being typed in the Files tree: a new file,
    /// a new folder or a rename.
    pub fn has_inline_edit(&self) -> bool {
        self.state.inline_edit().is_some()
    }

    /// Return whether the dialog that asks before a deletion is open.
    pub fn has_delete_dialog(&self) -> bool {
        self.state.pending_delete().is_some()
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

impl Component for Explorer {
    type Intent = ExplorerIntent;
    type Outcome = ExplorerOutcome;
    type Ctx<'a> = ExplorerCtx;

    fn reduce(
        &mut self,
        intent: ExplorerIntent,
        ctx: ExplorerCtx,
    ) -> Vec<ExplorerOutcome> {
        reducer::reduce(&mut self.state, intent, &ctx)
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

impl Overlays for Explorer {
    /// The dialog that asks before a deletion, or else the context menu.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, ExplorerIntent>> {
        if let Some(vm) = DeleteDialogViewModel::from_state(&self.state) {
            return Some(Overlay::dialog(delete_dialog::view(vm, theme)));
        }

        let vm = ContextMenuViewModel::from_state(&self.state)?;
        Some(Overlay::menu(context_menu::view(vm, theme)))
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_i18n::{self as i18n, Key};
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn ctx() -> ExplorerCtx {
        ExplorerCtx {
            cursor: Point::new(10.0, 10.0),
            screen_size: Size::new(800.0, 600.0),
        }
    }

    /// An explorer that follows `/tmp/project` and shows its file
    /// `main.rs`.
    fn explorer_with_file() -> Explorer {
        let root = PathBuf::from("/tmp/project");
        let mut explorer = Explorer::new();
        let _ = explorer
            .reduce(ExplorerIntent::SyncRoot { cwd: root.clone() }, ctx());
        let _ = explorer.reduce(
            ExplorerIntent::RootLoaded {
                root: root.clone(),
                nodes: vec![FileNode::new(
                    String::from("main.rs"),
                    root.join("main.rs"),
                    false,
                )],
            },
            ctx(),
        );

        explorer
    }

    fn overlay_kind(explorer: &Explorer) -> Option<OverlayKind> {
        let themes = ThemeManager::new();
        explorer
            .overlay(ThemeProps::new(themes.current()))
            .map(|overlay| overlay.kind())
    }

    /// The context menu is a menu; Delete puts the dialog in its place.
    #[test]
    fn the_menu_is_a_menu_and_the_delete_question_a_dialog() {
        let mut explorer = explorer_with_file();
        assert_eq!(overlay_kind(&explorer), None);

        let _ = explorer.reduce(
            ExplorerIntent::NodeRightClicked {
                path: vec![String::from("main.rs")],
            },
            ctx(),
        );
        assert!(explorer.has_context_menu());
        assert_eq!(overlay_kind(&explorer), Some(OverlayKind::Menu));

        let _ = explorer.reduce(
            ExplorerIntent::ContextMenuAction(ExplorerMenuAction::Delete),
            ctx(),
        );
        assert!(!explorer.has_context_menu());
        assert!(explorer.has_delete_dialog());
        assert_eq!(overlay_kind(&explorer), Some(OverlayKind::Dialog));
    }

    #[test]
    fn rename_from_the_menu_starts_the_name_field() {
        let mut explorer = explorer_with_file();
        let _ = explorer.reduce(
            ExplorerIntent::NodeRightClicked {
                path: vec![String::from("main.rs")],
            },
            ctx(),
        );

        let _ = explorer.reduce(
            ExplorerIntent::ContextMenuAction(ExplorerMenuAction::Rename),
            ctx(),
        );

        assert!(explorer.has_inline_edit());
        assert_eq!(overlay_kind(&explorer), None);
    }

    /// The menu opened on a file in the right panel shows its items.
    #[test]
    fn the_overlay_of_an_open_menu_shows_its_items() {
        let themes = ThemeManager::new();
        let mut explorer = explorer_with_file();
        let _ = explorer.reduce(
            ExplorerIntent::NodeRightClicked {
                path: vec![String::from("main.rs")],
            },
            ctx(),
        );
        let overlay = explorer
            .overlay(ThemeProps::new(themes.current()))
            .expect("the menu should be open");

        let mut ui = simulator(overlay.into_content());

        assert!(ui.find(i18n::t(Key::FilesOpen)).is_ok());
    }

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
            ctx(),
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
            ctx(),
        );

        let mut ui =
            simulator(explorer.right_panel(ThemeProps::new(themes.current())));

        assert!(ui.find("Files").is_ok());
        assert!(ui.find("project").is_ok());
    }
}
