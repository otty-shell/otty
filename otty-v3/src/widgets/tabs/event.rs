/// UI events emitted by tab bar views.
#[derive(Debug, Clone)]
pub(crate) enum TabsEvent {
    ActivateTab { tab_id: u64 },
    CloseTab { tab_id: u64 },
}

/// Effect events produced by the tabs reducer.
#[derive(Debug, Clone)]
pub(crate) enum TabsEffect {
    /// Tab activated; router coordinates terminal focus and explorer sync.
    Activated { tab_id: u64 },
    /// Tab closed with context for cross-widget cleanup.
    Closed {
        tab_id: u64,
        new_active_id: Option<u64>,
        remaining: usize,
    },
    /// Terminal tab opened; router creates terminal instance.
    TerminalTabOpened {
        tab_id: u64,
        terminal_id: u64,
        title: String,
    },
    /// Command tab opened; router creates command terminal instance.
    CommandTabOpened {
        tab_id: u64,
        terminal_id: u64,
        title: String,
        settings: Box<otty_ui_term::settings::Settings>,
    },
    /// Settings tab opened; router triggers settings reload.
    SettingsTabOpened,
    /// Wizard tab opened; flow router initializes the wizard state.
    WizardTabOpened { tab_id: u64 },
    /// Error tab opened; flow router initializes the error payload.
    ErrorTabOpened { tab_id: u64 },
    /// Tab bar should scroll to show the newest tab.
    ScrollBarToEnd,
}
