use std::path::PathBuf;

use otty_app_domain::exit_behavior::ExitBehavior;
use otty_app_domain::quick_launch::WizardTabInit;
use otty_app_domain::tab::TabMachine;
use otty_ui_term::settings::Settings;

/// What the tabs are asked to do: by the Sessions section, a shortcut, the
/// palette, the startup work, or another feature through the binary.
#[derive(Debug, Clone)]
pub enum TabsIntent {
    /// Make an open tab the active one (a press on its row or a palette
    /// pick).
    ActivateTab {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Close a tab (its cross, a middle click, or its terminal that ended).
    CloseTab {
        /// Id of the tab.
        tab_id: u64,
    },
    /// Close the active tab, if there is one.
    CloseActive,
    /// Name a terminal tab after its focused pane.
    SetTitle {
        /// Id of the tab.
        tab_id: u64,
        /// The new title.
        title: String,
        /// Folder name of the focused pane; `None` without one.
        folder: Option<String>,
    },
    /// Open a terminal tab running the user's shell, named after it.
    OpenTerminalTab {
        /// Folder the shell starts in, such as the one of Open in terminal
        /// in Files; `None` for the folder the shell session starts in, the
        /// home folder.
        folder: Option<PathBuf>,
    },
    /// Open a tab that runs a command: a launch or a file in the editor.
    OpenCommandTab {
        /// Title of the tab.
        title: String,
        /// Settings of the terminal that runs the command.
        settings: Box<Settings>,
        /// What happens to the tab when the command ends.
        exit_behavior: ExitBehavior,
        /// Machine the command runs on: the SSH host for an SSH launch.
        machine: TabMachine,
    },
    /// Open the launch editor in a tab.
    OpenWizardTab {
        /// Title of the tab.
        title: String,
        /// What the editor starts with.
        init: WizardTabInit,
    },
    /// This computer's host name was read; `None` when it could not be
    /// read. It names the group of this computer's tabs.
    HostNameRead(Option<String>),
}

/// What the tabs report back to the frame: news for the features that work
/// with a tab, and the Sessions list to scroll.
#[derive(Debug, Clone)]
pub enum TabsOutcome {
    /// A tab came to the front without waiting for its terminal: an open tab
    /// became active, or the launch editor opened. Tabs show only in
    /// Terminal mode, so another mode switches to it. A new terminal or
    /// command tab comes to the front once its terminal starts, which the
    /// terminal feature reports.
    Shown,
    /// An open tab became the active one: its terminal takes the keys, and
    /// the Files section follows its folder.
    Activated {
        /// Id of the tab.
        tab_id: u64,
    },
    /// A tab closed: what ran in it ends.
    Closed {
        /// Id of the closed tab.
        tab_id: u64,
        /// Id of the tab that is active now; `None` when no tab is left.
        new_active_id: Option<u64>,
    },
    /// A terminal tab opened and is active: start its shell.
    TerminalTabOpened {
        /// Id of the tab.
        tab_id: u64,
        /// Title of the tab, the shell's name.
        title: String,
        /// Folder the shell starts in; `None` for the home folder.
        folder: Option<PathBuf>,
    },
    /// A command tab opened and is active: start its command.
    CommandTabOpened {
        /// Id of the tab.
        tab_id: u64,
        /// Title of the tab.
        title: String,
        /// Settings of the terminal that runs the command.
        settings: Box<Settings>,
        /// What happens to the tab when the command ends.
        exit_behavior: ExitBehavior,
    },
    /// The launch editor opened in a tab and is active: fill the editor.
    WizardTabOpened {
        /// Id of the tab.
        tab_id: u64,
        /// What the editor starts with.
        init: WizardTabInit,
    },
    /// A tab was added at the end of the Sessions list: scroll the list to
    /// its end, so the new tab shows.
    ScrollSessionsToEnd,
}
