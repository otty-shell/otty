use iced::Task;

use crate::app::Event as AppEvent;
use crate::state::State;

use super::errors::SettingsError;
use super::model::SettingsData;
use super::state::{SettingsPreset, SettingsState, read_settings_payload};
#[cfg(test)]
use super::storage::SettingsLoadStatus;
use super::storage::{SettingsLoad, load_settings, save_settings};

/// UI and internal events handled by the settings feature reducer.
#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    Reload,
    Save,
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
        SettingsEvent::Reload => {
            reload_settings_state(&mut state.settings, load_settings)
        },
        SettingsEvent::Save => {
            persist_settings(&mut state.settings, save_settings)
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

fn reload_settings_state<Load>(
    state: &mut SettingsState,
    load: Load,
) -> Task<AppEvent>
where
    Load: Fn() -> Result<SettingsLoad, SettingsError>,
{
    if let Some(settings) = read_settings_payload(load) {
        state.replace_with_settings(settings);
    }

    Task::none()
}

fn persist_settings<Save>(
    state: &mut SettingsState,
    save: Save,
) -> Task<AppEvent>
where
    Save: Fn(&SettingsData) -> Result<(), SettingsError>,
{
    let normalized = state.normalized_draft();
    match save(&normalized) {
        Ok(()) => {
            state.mark_saved(normalized.clone());
            Task::done(AppEvent::SettingsApplied(normalized))
        },
        Err(err) => {
            log::warn!("settings save failed: {err}");
            Task::none()
        },
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;

    use crate::state::State;

    use super::{
        SettingsData, SettingsError, SettingsEvent, SettingsState,
        persist_settings, settings_reducer,
    };

    #[test]
    fn given_save_event_when_save_succeeds_then_marks_state_saved() {
        let mut settings_state = SettingsState::default();
        settings_state.set_shell(String::from("/bin/zsh"));
        assert!(settings_state.is_dirty());

        let _task = persist_settings(&mut settings_state, |_| Ok(()));

        assert!(!settings_state.is_dirty());
        assert_eq!(settings_state.baseline(), settings_state.draft());
    }

    #[test]
    fn given_save_event_when_save_fails_then_keeps_state_dirty() {
        let mut settings_state = SettingsState::default();
        settings_state.set_editor(String::from("vim"));
        assert!(settings_state.is_dirty());

        let _task = persist_settings(&mut settings_state, |_| {
            Err(SettingsError::Io(Error::other("save failed")))
        });

        assert!(settings_state.is_dirty());
        assert_ne!(settings_state.baseline(), settings_state.draft());
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

        let _task = super::reload_settings_state(&mut settings_state, || {
            Ok(super::SettingsLoad::new(
                loaded.clone(),
                super::SettingsLoadStatus::Loaded,
            ))
        });

        assert_eq!(settings_state.draft(), &loaded);
        assert!(!settings_state.is_dirty());
    }
}
