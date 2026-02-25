use iced::Task;

use super::model::SettingsData;
use super::state::{SettingsPreset, SettingsState};
use super::storage::{
    SettingsLoad, SettingsLoadStatus, load_settings, save_settings,
};
use crate::app::Event as AppEvent;
use crate::state::State;

/// UI and internal events handled by the settings feature reducer.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    Reload,
    ReloadLoaded(SettingsLoad),
    ReloadFailed(String),
    Save,
    SaveCompleted(SettingsData),
    SaveFailed(String),
    Reset,
    NodePressed { path: Vec<String> },
    NodeHovered { path: Option<Vec<String>> },
    ShellChanged(String),
    EditorChanged(String),
    PaletteChanged { index: usize, value: String },
    ApplyPreset(SettingsPreset),
}

/// Primary reducer entrypoint for settings events.
pub(crate) fn settings_reducer(
    state: &mut State,
    event: SettingsEvent,
) -> Task<AppEvent> {
    match event {
        SettingsEvent::Reload => request_reload_settings(),
        SettingsEvent::ReloadLoaded(load) => {
            apply_loaded_settings(&mut state.settings, load);
            Task::none()
        },
        SettingsEvent::ReloadFailed(message) => {
            log::warn!("settings read failed: {message}");
            Task::none()
        },
        SettingsEvent::Save => request_save_settings(&state.settings),
        SettingsEvent::SaveCompleted(settings) => {
            state.settings.mark_saved(settings.clone());
            Task::done(AppEvent::SettingsApplied(settings))
        },
        SettingsEvent::SaveFailed(message) => {
            log::warn!("settings save failed: {message}");
            Task::none()
        },
        SettingsEvent::Reset => {
            state.settings.reset();
            Task::none()
        },
        SettingsEvent::NodePressed { path } => {
            state.settings.select_path(&path);
            Task::none()
        },
        SettingsEvent::NodeHovered { path } => {
            state.settings.set_hovered_path(path);
            Task::none()
        },
        SettingsEvent::ShellChanged(value) => {
            state.settings.set_shell(value);
            Task::none()
        },
        SettingsEvent::EditorChanged(value) => {
            state.settings.set_editor(value);
            Task::none()
        },
        SettingsEvent::PaletteChanged { index, value } => {
            state.settings.set_palette_input(index, value);
            Task::none()
        },
        SettingsEvent::ApplyPreset(preset) => {
            state.settings.apply_preset(preset);
            Task::none()
        },
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
    use super::{
        SettingsData, SettingsEvent, SettingsState, apply_loaded_settings,
        settings_reducer,
    };
    use crate::state::State;

    #[test]
    fn given_save_event_when_save_succeeds_then_marks_state_saved() {
        let mut state = State::default();
        state.settings.set_shell(String::from("/bin/zsh"));
        assert!(state.settings.is_dirty());
        let normalized = state.settings.normalized_draft();

        let _task = settings_reducer(
            &mut state,
            SettingsEvent::SaveCompleted(normalized),
        );

        assert!(!state.settings.is_dirty());
        assert_eq!(state.settings.baseline(), state.settings.draft());
    }

    #[test]
    fn given_save_event_when_save_fails_then_keeps_state_dirty() {
        let mut state = State::default();
        state.settings.set_editor(String::from("vim"));
        assert!(state.settings.is_dirty());

        let _task = settings_reducer(
            &mut state,
            SettingsEvent::SaveFailed(String::from("save failed")),
        );

        assert!(state.settings.is_dirty());
        assert_ne!(state.settings.baseline(), state.settings.draft());
    }

    #[test]
    fn given_unknown_node_path_when_pressed_then_reducer_ignores_event() {
        let mut state = State::default();
        let selected_before = state.settings.selected_path().to_vec();
        let dirty_before = state.settings.is_dirty();

        let _task = settings_reducer(
            &mut state,
            SettingsEvent::NodePressed {
                path: vec![String::from("Unknown")],
            },
        );

        assert_eq!(state.settings.selected_path(), &selected_before);
        assert_eq!(state.settings.is_dirty(), dirty_before);
    }

    #[test]
    fn given_reload_event_when_load_succeeds_then_state_replaced() {
        let mut settings_state = SettingsState::default();
        settings_state.set_shell(String::from("/bin/zsh"));
        let loaded = SettingsData::default();

        apply_loaded_settings(
            &mut settings_state,
            super::SettingsLoad::new(
                loaded.clone(),
                super::SettingsLoadStatus::Loaded,
            ),
        );

        assert_eq!(settings_state.draft(), &loaded);
        assert!(!settings_state.is_dirty());
    }
}
