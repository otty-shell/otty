use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use iced::widget::pane_grid;
use otty_ui_kit::components::target_dot::TargetKind;

use super::services;
use super::state::PaneContextMenuState;
use super::types::TerminalEntry;

/// Longest folder path, counted after `~` or `/`, shown in full.
const SHORT_PATH_FOLDERS: usize = 3;
/// Folders kept at the end of a longer path.
const SHORT_PATH_TAIL: usize = 2;

/// View model for a single terminal tab, used by the pane grid view.
#[derive(Clone, Copy)]
pub(crate) struct TerminalTabViewModel<'a> {
    pub(crate) tab_id: u64,
    pub(crate) panes: &'a pane_grid::State<u64>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
    pub(crate) context_menu: Option<&'a PaneContextMenuState>,
    pub(crate) has_block_selection: bool,
    /// Name shown in the machine chip of every pane header: this
    /// computer's host name or the SSH host.
    pub(crate) machine: &'a str,
    /// Dot drawn in the machine chip.
    pub(crate) machine_target: TargetKind,
}

/// View model for the entire terminal workspace, keyed by active tab id.
#[derive(Clone, Copy)]
pub(crate) struct TerminalWorkspaceViewModel<'a> {
    pub(crate) tab: Option<TerminalTabViewModel<'a>>,
}

/// What a pane runs and where. The pane header, the status bar and the
/// session row show parts of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneLabel {
    program: String,
    folder_name: Option<String>,
    folder_path: Option<String>,
}

impl PaneLabel {
    /// Return the program name.
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// Return the last folder of the working directory (`~` for home), if
    /// the shell has reported one.
    pub(crate) fn folder_name(&self) -> Option<&str> {
        self.folder_name.as_deref()
    }

    /// Return the working directory, shortened to its last folders when
    /// long, if the shell has reported one.
    pub(crate) fn folder_path(&self) -> Option<&str> {
        self.folder_path.as_deref()
    }

    /// Build the label of a pane: the running command, or the name the pane
    /// opened with while the shell waits at the prompt.
    pub(crate) fn for_entry(entry: &TerminalEntry) -> Self {
        let blocks = entry.terminal().blocks();
        let program = services::running_program_from_blocks(&blocks)
            .unwrap_or(entry.title());
        let cwd = services::terminal_cwd_from_blocks(&blocks);
        let home = std::env::var_os("HOME").map(PathBuf::from);

        Self::new(program, cwd.as_deref(), home.as_deref())
    }

    /// Build the label. A folder inside `home` is written with `~`.
    pub(crate) fn new(
        program: &str,
        cwd: Option<&Path>,
        home: Option<&Path>,
    ) -> Self {
        Self {
            program: program.to_string(),
            folder_name: cwd.map(|cwd| folder_name(cwd, home)),
            folder_path: cwd.map(|cwd| short_folder_path(cwd, home)),
        }
    }
}

fn folder_name(cwd: &Path, home: Option<&Path>) -> String {
    if home == Some(cwd) {
        return String::from("~");
    }

    match cwd.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => cwd.display().to_string(),
    }
}

fn short_folder_path(cwd: &Path, home: Option<&Path>) -> String {
    let (start, rest) = match home.and_then(|home| cwd.strip_prefix(home).ok())
    {
        Some(rest) => ("~", rest),
        None => ("", cwd),
    };
    let folders: Vec<_> = rest
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy()),
            _ => None,
        })
        .collect();

    if folders.is_empty() {
        return match start {
            "" => cwd.display().to_string(),
            _ => String::from(start),
        };
    }
    if folders.len() <= SHORT_PATH_FOLDERS {
        return format!("{start}/{}", folders.join("/"));
    }

    let tail = folders[folders.len() - SHORT_PATH_TAIL..].join("/");
    format!("{start}/…/{tail}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PaneLabel;

    const HOME: &str = "/home/ann";

    fn label(cwd: &str) -> PaneLabel {
        PaneLabel::new("zsh", Some(Path::new(cwd)), Some(Path::new(HOME)))
    }

    #[test]
    fn given_folder_inside_home_when_label_built_then_home_is_tilde() {
        let label = label("/home/ann/projects/otty");

        assert_eq!(label.program(), "zsh");
        assert_eq!(label.folder_name(), Some("otty"));
        assert_eq!(label.folder_path(), Some("~/projects/otty"));
    }

    #[test]
    fn given_folder_equal_to_home_when_label_built_then_folder_is_tilde() {
        let label = label(HOME);

        assert_eq!(label.folder_name(), Some("~"));
        assert_eq!(label.folder_path(), Some("~"));
    }

    #[test]
    fn given_folder_outside_home_when_label_built_then_path_is_unchanged() {
        let label = label("/var/log");

        assert_eq!(label.folder_name(), Some("log"));
        assert_eq!(label.folder_path(), Some("/var/log"));
    }

    #[test]
    fn given_root_folder_when_label_built_then_folder_is_slash() {
        let label = label("/");

        assert_eq!(label.folder_name(), Some("/"));
        assert_eq!(label.folder_path(), Some("/"));
    }

    #[test]
    fn given_sibling_with_same_prefix_as_home_when_label_built_then_not_shortened()
     {
        let label = label("/home/anna");

        assert_eq!(label.folder_path(), Some("/home/anna"));
    }

    #[test]
    fn given_three_folders_after_home_when_label_built_then_path_is_full() {
        let label = label("/home/ann/projects/main/ottyrun");

        assert_eq!(label.folder_path(), Some("~/projects/main/ottyrun"));
    }

    #[test]
    fn given_deep_folder_inside_home_when_label_built_then_last_two_stay() {
        let label = label("/home/ann/projects/main/ottyrun/otty");

        assert_eq!(label.folder_name(), Some("otty"));
        assert_eq!(label.folder_path(), Some("~/…/ottyrun/otty"));
    }

    #[test]
    fn given_deep_folder_outside_home_when_label_built_then_last_two_stay() {
        let label = label("/var/lib/containers/storage/overlay");

        assert_eq!(label.folder_path(), Some("/…/storage/overlay"));
    }

    #[test]
    fn given_unknown_folder_when_label_built_then_folder_is_missing() {
        let label = PaneLabel::new("zsh", None, None);

        assert_eq!(label.folder_name(), None);
        assert_eq!(label.folder_path(), None);
    }
}
