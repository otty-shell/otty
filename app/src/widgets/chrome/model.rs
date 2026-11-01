use otty_ui_kit::components::target_dot::TargetKind;

use crate::widgets::terminal_workspace::model::PaneLabel;

/// View model for the chrome widget.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChromeViewModel {
    pub(crate) is_fullscreen: bool,
}

/// What the status bar needs from the rest of the app.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StatusBarInput<'a> {
    /// Machine of the active session, or this computer with no tabs.
    pub(crate) machine_name: &'a str,
    pub(crate) machine_target: TargetKind,
    /// Program and folder of the focused pane; `None` unless the active
    /// tab is a terminal tab.
    pub(crate) focused_pane: Option<&'a PaneLabel>,
    /// Number of panes in the active terminal tab.
    pub(crate) pane_count: Option<usize>,
    /// Number of open tabs.
    pub(crate) session_count: usize,
}

/// View model of the status bar at the bottom of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusBarViewModel {
    machine_name: String,
    machine_target: TargetKind,
    folder: Option<String>,
    pane_count: Option<usize>,
    session_count: usize,
}

impl StatusBarViewModel {
    /// Return the machine name.
    pub(crate) fn machine_name(&self) -> &str {
        &self.machine_name
    }

    /// Return the dot kind of the machine.
    pub(crate) fn machine_target(&self) -> TargetKind {
        self.machine_target
    }

    /// Return the folder path of the focused pane, with `~` for home and
    /// only the last folders of a long path.
    pub(crate) fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Return the pane count of the active terminal tab.
    pub(crate) fn pane_count(&self) -> Option<usize> {
        self.pane_count
    }

    /// Return the number of open sessions.
    pub(crate) fn session_count(&self) -> usize {
        self.session_count
    }

    /// Report whether no session is open (the empty state).
    pub(crate) fn is_empty(&self) -> bool {
        self.session_count == 0
    }

    /// Build the status bar data from the active tab and focused pane.
    pub(crate) fn new(input: StatusBarInput<'_>) -> Self {
        Self {
            machine_name: input.machine_name.to_string(),
            machine_target: input.machine_target,
            folder: input
                .focused_pane
                .and_then(|pane| pane.folder_path())
                .map(str::to_string),
            pane_count: input.pane_count,
            session_count: input.session_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn pane(cwd: &str) -> PaneLabel {
        PaneLabel::new(
            "zsh",
            Some(Path::new(cwd)),
            Some(Path::new("/home/ann")),
        )
    }

    fn input<'a>(
        focused_pane: Option<&'a PaneLabel>,
        pane_count: Option<usize>,
        session_count: usize,
    ) -> StatusBarInput<'a> {
        StatusBarInput {
            machine_name: "fedora",
            machine_target: TargetKind::Local,
            focused_pane,
            pane_count,
            session_count,
        }
    }

    #[test]
    fn given_terminal_tab_when_built_then_folder_path_and_counts_are_shown() {
        let pane = pane("/home/ann/projects/main/ottyrun/otty");

        let vm = StatusBarViewModel::new(input(Some(&pane), Some(2), 3));

        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.machine_target(), TargetKind::Local);
        assert_eq!(vm.folder(), Some("~/…/ottyrun/otty"));
        assert_eq!(vm.pane_count(), Some(2));
        assert_eq!(vm.session_count(), 3);
        assert!(!vm.is_empty());
    }

    #[test]
    fn given_settings_tab_when_built_then_no_pane_data_is_shown() {
        let vm = StatusBarViewModel::new(input(None, None, 2));

        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.folder(), None);
        assert_eq!(vm.pane_count(), None);
        assert_eq!(vm.session_count(), 2);
        assert!(!vm.is_empty());
    }

    #[test]
    fn given_no_tabs_when_built_then_bar_is_empty_with_machine() {
        let vm = StatusBarViewModel::new(input(None, None, 0));

        assert!(vm.is_empty());
        assert_eq!(vm.machine_name(), "fedora");
        assert_eq!(vm.pane_count(), None);
    }

    #[test]
    fn given_folder_change_when_rebuilt_then_new_folder_is_shown() {
        let before = pane("/home/ann");
        let after = pane("/var/log");

        let first = StatusBarViewModel::new(input(Some(&before), Some(1), 1));
        let second = StatusBarViewModel::new(input(Some(&after), Some(1), 1));

        assert_eq!(first.folder(), Some("~"));
        assert_eq!(second.folder(), Some("/var/log"));
    }

    #[test]
    fn given_split_then_close_when_rebuilt_then_pane_count_follows() {
        let pane = pane("/home/ann");

        let one = StatusBarViewModel::new(input(Some(&pane), Some(1), 1));
        let two = StatusBarViewModel::new(input(Some(&pane), Some(2), 1));
        let back = StatusBarViewModel::new(input(Some(&pane), Some(1), 1));

        assert_eq!(one.pane_count(), Some(1));
        assert_eq!(two.pane_count(), Some(2));
        assert_eq!(back.pane_count(), Some(1));
    }
}
