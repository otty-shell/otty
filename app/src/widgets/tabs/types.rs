use otty_ui_kit::components::target_dot::TargetKind;

use crate::i18n::{self, Key};

/// Discriminant identifying the content kind of a workspace tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TabContent {
    Terminal,
    Settings,
    QuickLaunchWizard,
    QuickLaunchError,
}

/// Machine a tab runs on. It is set once, when the tab opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TabMachine {
    /// This computer: shells, local launches, Settings, the launch editor
    /// and launch errors.
    ThisComputer,
    /// The host an SSH launch connects to.
    Ssh(String),
}

impl TabMachine {
    /// Return the name shown for the machine: the SSH host, or this
    /// computer's host name. `host_name` is `None` when the host name could
    /// not be read; this computer is then called "This computer".
    pub(crate) fn name<'a>(&'a self, host_name: Option<&'a str>) -> &'a str {
        match self {
            Self::ThisComputer => {
                host_name.unwrap_or(i18n::t(Key::ThisComputer))
            },
            Self::Ssh(host) => host,
        }
    }

    /// Return the kind of target dot drawn for the machine.
    pub(crate) fn target_kind(&self) -> TargetKind {
        match self {
            Self::ThisComputer => TargetKind::Local,
            Self::Ssh(_) => TargetKind::Ssh,
        }
    }
}

/// Metadata for a single tab entry.
pub(crate) struct TabItem {
    title: String,
    /// Folder name of the focused pane; `None` for other tabs or before the
    /// shell reports one.
    folder: Option<String>,
    content: TabContent,
    machine: TabMachine,
}

impl TabItem {
    /// Return tab title shown in the session list and the title bar.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Return the folder name shown after the title in the session list.
    pub(crate) fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    /// Return content discriminator.
    pub(crate) fn content(&self) -> TabContent {
        self.content
    }

    /// Return the machine the tab runs on.
    pub(crate) fn machine(&self) -> &TabMachine {
        &self.machine
    }

    /// Create tab metadata with its content kind and machine.
    pub(crate) fn new(
        title: String,
        content: TabContent,
        machine: TabMachine,
    ) -> Self {
        Self {
            title,
            folder: None,
            content,
            machine,
        }
    }

    /// Update tab title.
    pub(crate) fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Update the folder name shown after the title.
    pub(crate) fn set_folder(&mut self, folder: Option<String>) {
        self.folder = folder;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_this_computer_with_a_host_name_when_named_then_host_name_is_used()
    {
        let machine = TabMachine::ThisComputer;

        assert_eq!(machine.name(Some("fedora")), "fedora");
    }

    #[test]
    fn given_this_computer_without_a_host_name_when_named_then_fallback_is_used()
     {
        let machine = TabMachine::ThisComputer;

        assert_eq!(machine.name(None), i18n::t(Key::ThisComputer));
    }

    #[test]
    fn given_ssh_machine_when_named_then_its_host_is_used() {
        let machine = TabMachine::Ssh(String::from("laptop"));

        assert_eq!(machine.name(Some("fedora")), "laptop");
    }

    #[test]
    fn given_each_machine_when_target_read_then_it_matches_the_kind() {
        assert_eq!(TabMachine::ThisComputer.target_kind(), TargetKind::Local);
        assert_eq!(
            TabMachine::Ssh(String::from("laptop")).target_kind(),
            TargetKind::Ssh
        );
    }
}
