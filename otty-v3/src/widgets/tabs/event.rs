use otty_ui_term::settings::Settings;

/// Intent events handled by tab bar views and cross-widget tab workflows.
#[derive(Debug, Clone)]
pub(crate) enum TabsIntent {
    ActivateTab {
        tab_id: u64,
    },
    CloseTab {
        tab_id: u64,
    },
    SetTitle {
        tab_id: u64,
        title: String,
    },
    OpenTerminalTab {
        terminal_id: u64,
        title: String,
    },
    OpenCommandTab {
        terminal_id: u64,
        title: String,
        settings: Box<Settings>,
    },
    OpenSettingsTab,
    OpenWizardTab {
        title: String,
    },
    OpenErrorTab {
        title: String,
    },
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

/// Tabs event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum TabsEvent {
    /// Intent event reduced by the tabs widget.
    Intent(TabsIntent),
    /// External effect orchestrated by app-level routing.
    Effect(TabsEffect),
}
