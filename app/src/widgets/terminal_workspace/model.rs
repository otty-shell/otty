use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::widget::pane_grid;

use super::services;
use super::state::PaneContextMenuState;
use super::types::TerminalEntry;

/// View model for a single terminal tab, used by the pane grid view.
#[derive(Clone, Copy)]
pub(crate) struct TerminalTabViewModel<'a> {
    pub(crate) tab_id: u64,
    pub(crate) panes: &'a pane_grid::State<u64>,
    pub(crate) terminals: &'a HashMap<u64, TerminalEntry>,
    pub(crate) focus: Option<pane_grid::Pane>,
    pub(crate) context_menu: Option<&'a PaneContextMenuState>,
    pub(crate) has_block_selection: bool,
    /// Name shown in the machine chip of every pane header. It is "Local"
    /// until panes know their SSH host name.
    pub(crate) machine: &'a str,
}

/// View model for the entire terminal workspace, keyed by active tab id.
#[derive(Clone, Copy)]
pub(crate) struct TerminalWorkspaceViewModel<'a> {
    pub(crate) tab: Option<TerminalTabViewModel<'a>>,
}

/// Text shown in the header above one pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PaneHeaderViewModel {
    program: String,
    folder: Option<String>,
    machine: String,
}

impl PaneHeaderViewModel {
    /// Return the program name.
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    /// Return the working folder, if the shell has reported one.
    pub(crate) fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Return the machine name shown in the chip.
    pub(crate) fn machine(&self) -> &str {
        &self.machine
    }

    /// Build the header text for a pane from its terminal entry.
    pub(crate) fn for_entry(entry: &TerminalEntry, machine: &str) -> Self {
        let cwd =
            services::terminal_cwd_from_blocks(&entry.terminal().blocks());
        let home = std::env::var_os("HOME").map(PathBuf::from);

        Self::new(entry.title(), cwd.as_deref(), home.as_deref(), machine)
    }

    /// Build the header text. A folder inside `home` is written with `~`.
    pub(crate) fn new(
        program: &str,
        cwd: Option<&Path>,
        home: Option<&Path>,
        machine: &str,
    ) -> Self {
        Self {
            program: program.to_string(),
            folder: cwd.map(|cwd| display_folder(cwd, home)),
            machine: machine.to_string(),
        }
    }
}

fn display_folder(cwd: &Path, home: Option<&Path>) -> String {
    let Some(rest) = home.and_then(|home| cwd.strip_prefix(home).ok()) else {
        return cwd.display().to_string();
    };

    if rest.as_os_str().is_empty() {
        return String::from("~");
    }

    format!("~/{}", rest.display())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::PaneHeaderViewModel;

    #[test]
    fn given_folder_inside_home_when_header_built_then_home_is_tilde() {
        let header = PaneHeaderViewModel::new(
            "zsh",
            Some(Path::new("/home/ann/projects/otty")),
            Some(Path::new("/home/ann")),
            "Local",
        );

        assert_eq!(header.program(), "zsh");
        assert_eq!(header.folder(), Some("~/projects/otty"));
        assert_eq!(header.machine(), "Local");
    }

    #[test]
    fn given_folder_equal_to_home_when_header_built_then_folder_is_tilde() {
        let header = PaneHeaderViewModel::new(
            "zsh",
            Some(Path::new("/home/ann")),
            Some(Path::new("/home/ann")),
            "Local",
        );

        assert_eq!(header.folder(), Some("~"));
    }

    #[test]
    fn given_folder_outside_home_when_header_built_then_path_is_unchanged() {
        let header = PaneHeaderViewModel::new(
            "bash",
            Some(Path::new("/var/log")),
            Some(Path::new("/home/ann")),
            "Local",
        );

        assert_eq!(header.folder(), Some("/var/log"));
    }

    #[test]
    fn given_sibling_with_same_prefix_as_home_when_header_built_then_not_shortened()
     {
        let header = PaneHeaderViewModel::new(
            "bash",
            Some(Path::new("/home/anna")),
            Some(Path::new("/home/ann")),
            "Local",
        );

        assert_eq!(header.folder(), Some("/home/anna"));
    }

    #[test]
    fn given_unknown_folder_when_header_built_then_folder_is_missing() {
        let header = PaneHeaderViewModel::new("zsh", None, None, "Local");

        assert_eq!(header.folder(), None);
    }
}
