use otty_ui_term::settings::Settings;

use super::types::TabMachine;
use crate::domain::exit_behavior::ExitBehavior;
use crate::domain::quick_launch::WizardTabInit;

/// Intent events handled by the session list and cross-widget tab workflows.
#[derive(Debug, Clone)]
pub(crate) enum TabsIntent {
    ActivateTab {
        tab_id: u64,
    },
    CloseTab {
        tab_id: u64,
    },
    /// Close the active tab, if there is one.
    CloseActive,
    /// Name a terminal tab after its focused pane.
    SetTitle {
        tab_id: u64,
        title: String,
        folder: Option<String>,
    },
    /// Open a terminal tab running the user's shell, named after it.
    OpenTerminalTab,
    OpenCommandTab {
        title: String,
        settings: Box<Settings>,
        exit_behavior: ExitBehavior,
        /// Machine the command runs on: the SSH host for an SSH launch.
        machine: TabMachine,
    },
    OpenWizardTab {
        title: String,
        init: WizardTabInit,
    },
}

/// Effect events produced by the tabs reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabsEffect {
    /// Tab activated; the effects move terminal focus and sync the explorer.
    Activated { tab_id: u64 },
    /// Tab closed with context for cross-widget cleanup.
    Closed {
        tab_id: u64,
        new_active_id: Option<u64>,
        remaining: usize,
    },
    /// Terminal tab opened; the effects ask for its terminal.
    TerminalTabOpened { tab_id: u64, title: String },
    /// Command tab opened; the effects ask for its command terminal.
    CommandTabOpened {
        tab_id: u64,
        title: String,
        settings: Box<otty_ui_term::settings::Settings>,
        exit_behavior: ExitBehavior,
    },
    /// Wizard tab opened; the effects ask to initialize the wizard state.
    WizardTabOpened { tab_id: u64, init: WizardTabInit },
    /// The session list should scroll to its end to show the newest tab.
    ScrollBarToEnd,
}

/// Tabs event stream carried through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum TabsEvent {
    /// Intent event reduced by the tabs widget.
    Intent(TabsIntent),
    /// An effect, run by `effects/tabs.rs`.
    Effect(TabsEffect),
}
