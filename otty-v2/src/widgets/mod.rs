pub(crate) mod explorer;
pub(crate) mod quick_launch;
pub(crate) mod quick_launch_wizard;
pub(crate) mod settings;
pub(crate) mod sidebar;
pub(crate) mod tab;
pub(crate) mod terminal;

/// Root container for widgets-first app slices.
pub(crate) struct Widgets {
    explorer: explorer::ExplorerFeature,
    quick_launch: quick_launch::QuickLaunchFeature,
    quick_launch_wizard: quick_launch_wizard::QuickLaunchWizardFeature,
    sidebar: sidebar::SidebarWidget,
    settings: settings::SettingsFeature,
    tab: tab::TabFeature,
    terminal: terminal::TerminalFeature,
}

impl Widgets {
    /// Create a widgets container with the given initial settings state.
    pub(crate) fn new(settings_state: settings::SettingsState) -> Self {
        Self {
            explorer: explorer::ExplorerFeature::new(),
            quick_launch: quick_launch::QuickLaunchFeature::load(),
            quick_launch_wizard:
                quick_launch_wizard::QuickLaunchWizardFeature::new(),
            sidebar: sidebar::SidebarWidget::new(),
            settings: settings::SettingsFeature::new(settings_state),
            tab: tab::TabFeature::new(),
            terminal: terminal::TerminalFeature::new(),
        }
    }

    /// Return read-only access to explorer widget state and queries.
    pub(crate) fn explorer(&self) -> &explorer::ExplorerFeature {
        &self.explorer
    }

    /// Return mutable access for routing explorer events.
    pub(crate) fn explorer_mut(&mut self) -> &mut explorer::ExplorerFeature {
        &mut self.explorer
    }

    /// Return read-only access to quick launch widget state and queries.
    pub(crate) fn quick_launch(&self) -> &quick_launch::QuickLaunchFeature {
        &self.quick_launch
    }

    /// Return mutable access for routing quick launch events.
    pub(crate) fn quick_launch_mut(
        &mut self,
    ) -> &mut quick_launch::QuickLaunchFeature {
        &mut self.quick_launch
    }

    /// Return read-only access to quick launch wizard widget state and queries.
    pub(crate) fn quick_launch_wizard(
        &self,
    ) -> &quick_launch_wizard::QuickLaunchWizardFeature {
        &self.quick_launch_wizard
    }

    /// Return mutable access for routing quick launch wizard events.
    pub(crate) fn quick_launch_wizard_mut(
        &mut self,
    ) -> &mut quick_launch_wizard::QuickLaunchWizardFeature {
        &mut self.quick_launch_wizard
    }

    /// Return read-only access to sidebar widget state and queries.
    pub(crate) fn sidebar(&self) -> &sidebar::SidebarWidget {
        &self.sidebar
    }

    /// Return mutable access for routing sidebar events.
    pub(crate) fn sidebar_mut(&mut self) -> &mut sidebar::SidebarWidget {
        &mut self.sidebar
    }

    /// Return read-only access to settings widget state and queries.
    pub(crate) fn settings(&self) -> &settings::SettingsFeature {
        &self.settings
    }

    /// Return mutable access for routing settings events.
    pub(crate) fn settings_mut(&mut self) -> &mut settings::SettingsFeature {
        &mut self.settings
    }

    /// Return read-only access to tab widget state and queries.
    pub(crate) fn tab(&self) -> &tab::TabFeature {
        &self.tab
    }

    /// Return mutable access for routing tab events.
    pub(crate) fn tab_mut(&mut self) -> &mut tab::TabFeature {
        &mut self.tab
    }

    /// Return read-only access to terminal widget state and queries.
    pub(crate) fn terminal(&self) -> &terminal::TerminalFeature {
        &self.terminal
    }

    /// Return mutable access for routing terminal events.
    pub(crate) fn terminal_mut(&mut self) -> &mut terminal::TerminalFeature {
        &mut self.terminal
    }
}
