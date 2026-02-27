pub(crate) mod command;
mod errors;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod reducer;
pub(crate) mod state;
pub(crate) mod storage;
pub(crate) mod view;

pub(crate) use command::SettingsCommand;
pub(crate) use event::SettingsEffect;
use iced::Task;
use model::{SettingsData, SettingsViewModel};
use state::SettingsState;

/// Settings widget: manages application settings (terminal shell/editor,
/// theme palette colors) with a draft/baseline editing pattern.
pub(crate) struct SettingsWidget {
    state: SettingsState,
}

impl SettingsWidget {
    /// Create a new widget with default state.
    pub(crate) fn new() -> Self {
        Self {
            state: SettingsState::default(),
        }
    }

    /// Create a widget by loading persisted state from disk.
    pub(crate) fn load() -> Self {
        Self {
            state: storage::load_initial_settings_state(),
        }
    }

    /// Reduce a command into state updates and effects.
    pub(crate) fn reduce(
        &mut self,
        command: SettingsCommand,
    ) -> Task<SettingsEffect> {
        reducer::reduce(&mut self.state, command)
    }

    /// Return a read-only view model for the settings form.
    pub(crate) fn vm(&self) -> SettingsViewModel<'_> {
        SettingsViewModel {
            draft: self.state.draft(),
            palette_inputs: self.state.palette_inputs(),
            tree: self.state.tree(),
            selected_section: self.state.selected_section(),
            selected_path: self.state.selected_path(),
            hovered_path: self.state.hovered_path(),
            is_dirty: self.state.is_dirty(),
        }
    }

    /// Return the configured terminal shell command.
    pub(crate) fn terminal_shell(&self) -> &str {
        self.state.draft().terminal_shell()
    }

    /// Return the configured terminal editor command.
    pub(crate) fn terminal_editor(&self) -> &str {
        self.state.draft().terminal_editor()
    }

    /// Return current settings draft used by app-level orchestration.
    pub(crate) fn settings_data(&self) -> &SettingsData {
        self.state.draft()
    }

    /// Return whether the draft differs from the persisted baseline.
    pub(crate) fn is_dirty(&self) -> bool {
        self.state.is_dirty()
    }

    /// Return read-only access to state for tests.
    #[cfg(test)]
    pub(crate) fn state(&self) -> &SettingsState {
        &self.state
    }
}
