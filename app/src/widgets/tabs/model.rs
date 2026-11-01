use std::collections::BTreeMap;

use otty_ui_kit::components::target_dot::TargetKind;

use super::types::{TabContent, TabItem, TabMachine};

/// View model of the Sessions section: the open tabs grouped by machine.
#[derive(Debug, Clone)]
pub(crate) struct TabsViewModel<'a> {
    /// Machine groups in the order their first open tab was opened.
    pub(crate) groups: Vec<SessionGroupViewModel<'a>>,
    /// Number of open tabs.
    pub(crate) session_count: usize,
}

impl<'a> TabsViewModel<'a> {
    /// Group `tabs` by machine. Groups keep the order in which their first
    /// open tab was opened; rows keep tab order. `host_name` names this
    /// computer (`None` when it could not be read).
    pub(crate) fn new(
        tabs: &'a BTreeMap<u64, TabItem>,
        active_tab_id: Option<u64>,
        host_name: Option<&'a str>,
    ) -> Self {
        let mut machines: Vec<&TabMachine> = Vec::new();
        let mut groups: Vec<SessionGroupViewModel<'a>> = Vec::new();

        for (&tab_id, tab) in tabs {
            let session = SessionViewModel {
                tab_id,
                title: tab.title(),
                folder: tab.folder(),
                state: SessionState::for_content(tab.content()),
                is_active: active_tab_id == Some(tab_id),
            };
            let machine = tab.machine();

            match machines.iter().position(|known| *known == machine) {
                Some(index) => groups[index].sessions.push(session),
                None => {
                    machines.push(machine);
                    groups.push(SessionGroupViewModel {
                        name: machine.name(host_name),
                        target: machine.target_kind(),
                        sessions: vec![session],
                    });
                },
            }
        }

        Self {
            groups,
            session_count: tabs.len(),
        }
    }
}

/// One machine and its sessions.
#[derive(Debug, Clone)]
pub(crate) struct SessionGroupViewModel<'a> {
    /// Host name of this computer or the SSH host.
    pub(crate) name: &'a str,
    pub(crate) target: TargetKind,
    pub(crate) sessions: Vec<SessionViewModel<'a>>,
}

/// One open tab in the Sessions section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionViewModel<'a> {
    pub(crate) tab_id: u64,
    pub(crate) title: &'a str,
    /// Folder name shown muted after the title; `None` without one.
    pub(crate) folder: Option<&'a str>,
    /// State word shown after the title; `None` for a terminal.
    pub(crate) state: Option<SessionState>,
    pub(crate) is_active: bool,
}

/// What a non-terminal session row says about its tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionState {
    /// The Settings tab.
    Settings,
    /// The launch editor.
    Editor,
    /// A launch that failed to start.
    Failed,
}

impl SessionState {
    /// Return the state shown for a tab with `content`.
    pub(crate) fn for_content(content: TabContent) -> Option<Self> {
        match content {
            TabContent::Settings => Some(Self::Settings),
            TabContent::QuickLaunchWizard => Some(Self::Editor),
            TabContent::QuickLaunchError => Some(Self::Failed),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{self, Key};

    fn tab(
        id: u64,
        title: &str,
        content: TabContent,
        machine: TabMachine,
    ) -> (u64, TabItem) {
        (id, TabItem::new(String::from(title), content, machine))
    }

    fn local_shell(id: u64, title: &str) -> (u64, TabItem) {
        tab(id, title, TabContent::Terminal, TabMachine::ThisComputer)
    }

    fn ssh_shell(id: u64, title: &str, host: &str) -> (u64, TabItem) {
        tab(
            id,
            title,
            TabContent::Terminal,
            TabMachine::Ssh(String::from(host)),
        )
    }

    fn group_names<'a>(vm: &TabsViewModel<'a>) -> Vec<&'a str> {
        vm.groups.iter().map(|group| group.name).collect()
    }

    fn titles<'a>(group: &SessionGroupViewModel<'a>) -> Vec<&'a str> {
        group.sessions.iter().map(|session| session.title).collect()
    }

    #[test]
    fn given_no_tabs_when_built_then_there_are_no_groups() {
        let tabs = BTreeMap::new();

        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));

        assert!(vm.groups.is_empty());
        assert_eq!(vm.session_count, 0);
    }

    #[test]
    fn given_local_and_ssh_tabs_when_built_then_tabs_are_grouped_by_machine() {
        let tabs = BTreeMap::from([
            local_shell(0, "zsh"),
            ssh_shell(1, "htop", "laptop"),
            local_shell(2, "claude"),
            ssh_shell(3, "logs", "laptop"),
        ]);

        let vm = TabsViewModel::new(&tabs, Some(0), Some("fedora"));

        assert_eq!(group_names(&vm), vec!["fedora", "laptop"]);
        assert_eq!(vm.groups[0].target, TargetKind::Local);
        assert_eq!(vm.groups[1].target, TargetKind::Ssh);
        assert_eq!(titles(&vm.groups[0]), vec!["zsh", "claude"]);
        assert_eq!(titles(&vm.groups[1]), vec!["htop", "logs"]);
        assert_eq!(vm.session_count, 4);
    }

    #[test]
    fn given_tab_with_folder_when_built_then_its_session_shows_the_folder() {
        let (id, mut shell) = local_shell(0, "zsh");
        shell.set_folder(Some(String::from("otty")));
        let tabs = BTreeMap::from([(id, shell), local_shell(1, "htop")]);

        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));

        let folders: Vec<Option<&str>> = vm.groups[0]
            .sessions
            .iter()
            .map(|session| session.folder)
            .collect();
        assert_eq!(folders, vec![Some("otty"), None]);
    }

    #[test]
    fn given_ssh_tab_opened_first_when_built_then_its_group_comes_first() {
        let tabs = BTreeMap::from([
            ssh_shell(4, "htop", "laptop"),
            local_shell(7, "zsh"),
            ssh_shell(9, "top", "server"),
        ]);

        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));

        assert_eq!(group_names(&vm), vec!["laptop", "fedora", "server"]);
    }

    #[test]
    fn given_two_ssh_hosts_when_built_then_each_host_has_its_own_group() {
        let tabs = BTreeMap::from([
            ssh_shell(0, "a", "laptop"),
            ssh_shell(1, "b", "server"),
            ssh_shell(2, "c", "laptop"),
        ]);

        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));

        assert_eq!(group_names(&vm), vec!["laptop", "server"]);
        assert_eq!(titles(&vm.groups[0]), vec!["a", "c"]);
    }

    #[test]
    fn given_unknown_host_name_when_built_then_this_computer_uses_the_fallback()
    {
        let tabs = BTreeMap::from([local_shell(0, "zsh")]);

        let vm = TabsViewModel::new(&tabs, None, None);

        assert_eq!(group_names(&vm), vec![i18n::t(Key::ThisComputer)]);
    }

    #[test]
    fn given_settings_editor_and_error_tabs_when_built_then_rows_have_state_words()
     {
        let tabs = BTreeMap::from([
            local_shell(0, "zsh"),
            tab(
                1,
                "Settings",
                TabContent::Settings,
                TabMachine::ThisComputer,
            ),
            tab(
                2,
                "Edit launch",
                TabContent::QuickLaunchWizard,
                TabMachine::ThisComputer,
            ),
            tab(
                3,
                "Launch failed",
                TabContent::QuickLaunchError,
                TabMachine::ThisComputer,
            ),
        ]);

        let vm = TabsViewModel::new(&tabs, None, Some("fedora"));

        let states: Vec<Option<SessionState>> = vm.groups[0]
            .sessions
            .iter()
            .map(|session| session.state)
            .collect();
        assert_eq!(
            states,
            vec![
                None,
                Some(SessionState::Settings),
                Some(SessionState::Editor),
                Some(SessionState::Failed),
            ]
        );
    }

    #[test]
    fn given_active_tab_when_built_then_only_its_row_is_active() {
        let tabs = BTreeMap::from([
            local_shell(0, "zsh"),
            ssh_shell(1, "htop", "laptop"),
        ]);

        let vm = TabsViewModel::new(&tabs, Some(1), Some("fedora"));

        assert_eq!(
            vm.groups[1].sessions,
            vec![SessionViewModel {
                tab_id: 1,
                title: "htop",
                folder: None,
                state: None,
                is_active: true,
            }]
        );
        assert!(!vm.groups[0].sessions[0].is_active);
    }
}
