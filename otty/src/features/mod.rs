use iced::Task;

use crate::app::Event as AppEvent;

pub(crate) mod explorer;
pub(crate) mod quick_launch;
pub(crate) mod quick_launch_wizard;
pub(crate) mod settings;
pub(crate) mod terminal;

/// Shared feature contract for stateful domain modules.
pub(crate) trait Feature {
    type Event;
    type Ctx<'a>
    where
        Self: 'a;

    /// Reduce a typed feature event into state mutations and routed app tasks.
    fn reduce<'a>(
        &mut self,
        event: Self::Event,
        ctx: &Self::Ctx<'a>,
    ) -> Task<AppEvent>;
}

/// Root container for migrated struct-based features.
pub(crate) struct Features {
    explorer: explorer::ExplorerFeature,
    quick_launch: quick_launch::QuickLaunchFeature,
    quick_launch_wizard: quick_launch_wizard::QuickLaunchWizardFeature,
    settings: settings::SettingsFeature,
    terminal: terminal::TerminalFeature,
}

impl Features {
    /// Create a features container with the given initial settings state.
    pub(crate) fn new(settings_state: settings::SettingsState) -> Self {
        Self {
            explorer: explorer::ExplorerFeature::new(),
            quick_launch: quick_launch::QuickLaunchFeature::load(),
            quick_launch_wizard:
                quick_launch_wizard::QuickLaunchWizardFeature::new(),
            settings: settings::SettingsFeature::new(settings_state),
            terminal: terminal::TerminalFeature::new(),
        }
    }

    /// Return read-only access to explorer feature state and queries.
    pub(crate) fn explorer(&self) -> &explorer::ExplorerFeature {
        &self.explorer
    }

    /// Return mutable access for routing explorer events.
    pub(crate) fn explorer_mut(&mut self) -> &mut explorer::ExplorerFeature {
        &mut self.explorer
    }

    /// Return read-only access to quick launch feature state and queries.
    pub(crate) fn quick_launch(&self) -> &quick_launch::QuickLaunchFeature {
        &self.quick_launch
    }

    /// Return mutable access for routing quick launch events.
    pub(crate) fn quick_launch_mut(
        &mut self,
    ) -> &mut quick_launch::QuickLaunchFeature {
        &mut self.quick_launch
    }

    /// Return read-only access to quick launch wizard feature state and queries.
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

    /// Return read-only access to settings feature state and queries.
    pub(crate) fn settings(&self) -> &settings::SettingsFeature {
        &self.settings
    }

    /// Return mutable access for routing settings events.
    pub(crate) fn settings_mut(&mut self) -> &mut settings::SettingsFeature {
        &mut self.settings
    }

    /// Return read-only access to terminal feature state and queries.
    pub(crate) fn terminal(&self) -> &terminal::TerminalFeature {
        &self.terminal
    }

    /// Return mutable access for routing terminal events.
    pub(crate) fn terminal_mut(&mut self) -> &mut terminal::TerminalFeature {
        &mut self.terminal
    }
}
