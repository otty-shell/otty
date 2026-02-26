use iced::Task;

use super::command::SettingsCommand;
use super::event::SettingsEffect;
use super::state::SettingsState;
use super::storage::{
    SettingsLoad, SettingsLoadStatus, load_settings, save_settings,
};

/// Reduce a settings command into state updates and effect tasks.
pub(crate) fn reduce(
    state: &mut SettingsState,
    command: SettingsCommand,
) -> Task<SettingsEffect> {
    match command {
        SettingsCommand::Reload => request_reload_settings(),
        SettingsCommand::ReloadLoaded(load) => {
            apply_loaded_settings(state, load);
            Task::none()
        },
        SettingsCommand::ReloadFailed(message) => {
            log::warn!("settings read failed: {message}");
            Task::none()
        },
        SettingsCommand::Save => request_save_settings(state),
        SettingsCommand::SaveCompleted(settings) => {
            state.mark_saved(settings.clone());
            Task::done(SettingsEffect::SaveCompleted(settings))
        },
        SettingsCommand::SaveFailed(message) => {
            log::warn!("settings save failed: {message}");
            Task::none()
        },
        SettingsCommand::Reset => {
            state.reset();
            Task::none()
        },
        SettingsCommand::NodePressed { path } => {
            state.select_path(&path);
            Task::none()
        },
        SettingsCommand::NodeHovered { path } => {
            state.set_hovered_path(path);
            Task::none()
        },
        SettingsCommand::ShellChanged(value) => {
            state.set_shell(value);
            Task::none()
        },
        SettingsCommand::EditorChanged(value) => {
            state.set_editor(value);
            Task::none()
        },
        SettingsCommand::PaletteChanged { index, value } => {
            state.set_palette_input(index, value);
            Task::none()
        },
        SettingsCommand::ApplyPreset(preset) => {
            state.apply_preset(preset);
            Task::none()
        },
    }
}

fn request_reload_settings() -> Task<SettingsEffect> {
    Task::perform(async { load_settings() }, |result| match result {
        Ok(load) => SettingsEffect::ApplyTheme(load.into_parts().0),
        Err(err) => {
            log::warn!("settings reload failed: {err}");
            // On reload failure we cannot emit a meaningful effect;
            // the caller should have routed ReloadFailed instead.
            // We emit ApplyTheme with defaults so the chain continues.
            SettingsEffect::ApplyTheme(super::model::SettingsData::default())
        },
    })
}

fn request_save_settings(state: &SettingsState) -> Task<SettingsEffect> {
    let normalized = state.normalized_draft();
    Task::perform(
        async move {
            match save_settings(&normalized) {
                Ok(()) => Ok(normalized),
                Err(err) => Err(format!("{err}")),
            }
        },
        |result| match result {
            Ok(settings) => SettingsEffect::SaveCompleted(settings),
            Err(message) => {
                log::warn!("settings save failed: {message}");
                // Bubble a save-completed with the default so the
                // router can decide what to do.
                SettingsEffect::SaveCompleted(
                    super::model::SettingsData::default(),
                )
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
    use super::*;
    use crate::widgets::settings::command::SettingsCommand;
    use crate::widgets::settings::model::{
        SettingsData, SettingsPreset, SettingsSection,
    };
    use crate::widgets::settings::state::SettingsState;
    use crate::widgets::settings::storage::{SettingsLoad, SettingsLoadStatus};

    fn default_state() -> SettingsState {
        SettingsState::default()
    }

    #[test]
    fn given_save_completed_when_reduced_then_marks_state_saved() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/zsh"));
        assert!(state.is_dirty());
        let normalized = state.normalized_draft();

        let _task =
            reduce(&mut state, SettingsCommand::SaveCompleted(normalized));

        assert!(!state.is_dirty());
        assert_eq!(state.baseline(), state.draft());
    }

    #[test]
    fn given_save_failed_when_reduced_then_keeps_state_dirty() {
        let mut state = default_state();
        state.set_editor(String::from("vim"));
        assert!(state.is_dirty());

        let _task = reduce(
            &mut state,
            SettingsCommand::SaveFailed(String::from("write failed")),
        );

        assert!(state.is_dirty());
        assert_ne!(state.baseline(), state.draft());
    }

    #[test]
    fn given_unknown_node_path_when_pressed_then_reducer_ignores_event() {
        let mut state = default_state();
        let selected_before = state.selected_path().to_vec();
        let dirty_before = state.is_dirty();

        let _task = reduce(
            &mut state,
            SettingsCommand::NodePressed {
                path: vec![String::from("Unknown")],
            },
        );

        assert_eq!(state.selected_path(), &selected_before);
        assert_eq!(state.is_dirty(), dirty_before);
    }

    #[test]
    fn given_reload_event_when_load_succeeds_then_state_replaced() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/zsh"));
        let loaded = SettingsData::default();

        apply_loaded_settings(
            &mut state,
            SettingsLoad::new(loaded.clone(), SettingsLoadStatus::Loaded),
        );

        assert_eq!(state.draft(), &loaded);
        assert!(!state.is_dirty());
    }

    #[test]
    fn given_reset_command_when_reduced_then_draft_matches_baseline() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/fish"));
        state.set_editor(String::from("nvim"));
        assert!(state.is_dirty());

        let _task = reduce(&mut state, SettingsCommand::Reset);

        assert!(!state.is_dirty());
        assert_eq!(state.draft(), state.baseline());
    }

    #[test]
    fn given_shell_changed_when_reduced_then_draft_shell_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsCommand::ShellChanged(String::from("/usr/bin/fish")),
        );

        assert_eq!(state.draft().terminal_shell(), "/usr/bin/fish");
        assert!(state.is_dirty());
    }

    #[test]
    fn given_palette_changed_when_reduced_then_palette_input_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsCommand::PaletteChanged {
                index: 0,
                value: String::from("#AABB"),
            },
        );

        assert_eq!(state.palette_inputs()[0], "#AABB");
        // Incomplete hex should not update the draft palette
        assert_ne!(state.draft().theme_palette()[0], "#AABB");
    }

    #[test]
    fn given_apply_preset_when_reduced_then_palette_matches_preset() {
        let mut baseline = SettingsData::default();
        baseline.set_theme_palette_entry(0, String::from("#999999"));
        let mut state = SettingsState::from_settings(baseline);

        let _task = reduce(
            &mut state,
            SettingsCommand::ApplyPreset(SettingsPreset::OttyDark),
        );

        let expected = SettingsPreset::OttyDark.palette();
        assert_eq!(state.draft().theme_palette(), &expected[..]);
        assert!(state.is_dirty());
    }

    #[test]
    fn given_node_pressed_with_section_then_selection_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsCommand::NodePressed {
                path: vec![String::from("General"), String::from("Theme")],
            },
        );

        assert_eq!(state.selected_section(), SettingsSection::Theme);
    }
}
