use iced::Task;

use super::event::SettingsEvent;
use super::state::SettingsState;
use super::storage::{
    SettingsLoad, SettingsLoadStatus, load_settings, save_settings,
};
use crate::app::Event as AppEvent;

/// Settings feature root that owns settings state and reduction logic.
#[derive(Debug)]
pub(crate) struct SettingsFeature {
    state: SettingsState,
}

impl SettingsFeature {
    /// Construct the settings feature with the given initial state.
    pub(crate) fn new(state: SettingsState) -> Self {
        Self { state }
    }

    /// Return read-only access to settings state for the view layer.
    pub(crate) fn state(&self) -> &SettingsState {
        &self.state
    }

    /// Return the configured terminal editor command.
    pub(crate) fn terminal_editor(&self) -> &str {
        self.state.draft().terminal_editor()
    }
}

impl SettingsFeature {
    /// Reduce a settings event into state updates and routed app tasks.
    pub(crate) fn reduce(
        &mut self,
        event: SettingsEvent,
        _ctx: &(),
    ) -> Task<AppEvent> {
        match event {
            SettingsEvent::Reload => request_reload_settings(),
            SettingsEvent::ReloadLoaded(load) => {
                apply_loaded_settings(&mut self.state, load);
                Task::none()
            },
            SettingsEvent::ReloadFailed(message) => {
                log::warn!("settings read failed: {message}");
                Task::none()
            },
            SettingsEvent::Save => request_save_settings(&self.state),
            SettingsEvent::SaveCompleted(settings) => {
                self.state.mark_saved(settings.clone());
                Task::done(AppEvent::SettingsApplied(settings))
            },
            SettingsEvent::SaveFailed(message) => {
                log::warn!("settings save failed: {message}");
                Task::none()
            },
            SettingsEvent::Reset => {
                self.state.reset();
                Task::none()
            },
            SettingsEvent::NodePressed { path } => {
                self.state.select_path(&path);
                Task::none()
            },
            SettingsEvent::NodeHovered { path } => {
                self.state.set_hovered_path(path);
                Task::none()
            },
            SettingsEvent::ShellChanged(value) => {
                self.state.set_shell(value);
                Task::none()
            },
            SettingsEvent::EditorChanged(value) => {
                self.state.set_editor(value);
                Task::none()
            },
            SettingsEvent::PaletteChanged { index, value } => {
                self.state.set_palette_input(index, value);
                Task::none()
            },
            SettingsEvent::ApplyPreset(preset) => {
                self.state.apply_preset(preset);
                Task::none()
            },
        }
    }
}

fn request_reload_settings() -> Task<AppEvent> {
    Task::perform(async { load_settings() }, |result| match result {
        Ok(load) => AppEvent::Settings(SettingsEvent::ReloadLoaded(load)),
        Err(err) => {
            AppEvent::Settings(SettingsEvent::ReloadFailed(format!("{err}")))
        },
    })
}

fn request_save_settings(state: &SettingsState) -> Task<AppEvent> {
    let normalized = state.normalized_draft();
    Task::perform(
        async move {
            match save_settings(&normalized) {
                Ok(()) => Ok(normalized),
                Err(err) => Err(format!("{err}")),
            }
        },
        |result| match result {
            Ok(settings) => {
                AppEvent::Settings(SettingsEvent::SaveCompleted(settings))
            },
            Err(message) => {
                AppEvent::Settings(SettingsEvent::SaveFailed(message))
            },
        },
    )
}

fn apply_loaded_settings(state: &mut SettingsState, load: SettingsLoad) {
    let (settings, status) = load.into_parts();
    if let SettingsLoadStatus::Invalid(message) = &status {
        log::warn!("settings file invalid: {message}");
    }

    state.replace_with_settings(settings);
}

#[cfg(test)]
mod tests {
    use super::{SettingsFeature, apply_loaded_settings};
    use crate::features::settings::event::SettingsEvent;
    use crate::features::settings::model::SettingsData;
    use crate::features::settings::state::SettingsState;
    use crate::features::settings::storage::{
        SettingsLoad, SettingsLoadStatus,
    };

    fn feature() -> SettingsFeature {
        SettingsFeature::new(SettingsState::default())
    }

    #[test]
    fn given_save_event_when_save_succeeds_then_marks_state_saved() {
        let mut f = feature();
        f.state.set_shell(String::from("/bin/zsh"));
        assert!(f.state.is_dirty());
        let normalized = f.state.normalized_draft();

        let _task = f.reduce(SettingsEvent::SaveCompleted(normalized), &());

        assert!(!f.state.is_dirty());
        assert_eq!(f.state.baseline(), f.state.draft());
    }

    #[test]
    fn given_save_event_when_save_fails_then_keeps_state_dirty() {
        let mut f = feature();
        f.state.set_editor(String::from("vim"));
        assert!(f.state.is_dirty());

        let _task = f.reduce(
            SettingsEvent::SaveFailed(String::from("save failed")),
            &(),
        );

        assert!(f.state.is_dirty());
        assert_ne!(f.state.baseline(), f.state.draft());
    }

    #[test]
    fn given_unknown_node_path_when_pressed_then_reducer_ignores_event() {
        let mut f = feature();
        let selected_before = f.state.selected_path().to_vec();
        let dirty_before = f.state.is_dirty();

        let _task = f.reduce(
            SettingsEvent::NodePressed {
                path: vec![String::from("Unknown")],
            },
            &(),
        );

        assert_eq!(f.state.selected_path(), &selected_before);
        assert_eq!(f.state.is_dirty(), dirty_before);
    }

    #[test]
    fn given_reload_event_when_load_succeeds_then_state_replaced() {
        let mut settings_state = SettingsState::default();
        settings_state.set_shell(String::from("/bin/zsh"));
        let loaded = SettingsData::default();

        apply_loaded_settings(
            &mut settings_state,
            SettingsLoad::new(loaded.clone(), SettingsLoadStatus::Loaded),
        );

        assert_eq!(settings_state.draft(), &loaded);
        assert!(!settings_state.is_dirty());
    }
}
