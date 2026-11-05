#![forbid(clippy::disallowed_methods)]

use iced::Task;

use super::event::{SettingsEffect, SettingsEvent, SettingsIntent};
use super::state::SettingsState;
use super::types::{SettingsData, SettingsLoad, SettingsLoadStatus};
use crate::i18n;

/// Reduce a settings intent event into state updates and effect tasks.
pub(crate) fn reduce(
    state: &mut SettingsState,
    event: SettingsIntent,
) -> Task<SettingsEvent> {
    if state.is_saving() && changes_draft(&event) {
        return Task::none();
    }

    match event {
        SettingsIntent::Reload => {
            Task::done(SettingsEvent::Effect(SettingsEffect::LoadRequested))
        },
        SettingsIntent::ReloadLoaded(load) => {
            let settings = apply_loaded_settings(state, load);
            Task::done(SettingsEvent::Effect(SettingsEffect::ApplyTheme(
                settings,
            )))
        },
        SettingsIntent::ReloadFailed(message) => {
            log::warn!("settings read failed: {message}");
            Task::none()
        },
        SettingsIntent::Save => request_save_settings(state),
        SettingsIntent::SaveCompleted(settings) => {
            i18n::set_locale(settings.language().resolve());
            state.mark_saved(settings.clone());
            Task::done(SettingsEvent::Effect(SettingsEffect::ApplyTheme(
                settings,
            )))
        },
        SettingsIntent::SaveFailed(message) => {
            state.mark_save_failed();
            log::warn!("settings save failed: {message}");
            Task::none()
        },
        SettingsIntent::Reset => {
            state.reset();
            Task::none()
        },
        SettingsIntent::LanguageChanged(value) => {
            state.set_language(value);
            Task::none()
        },
        SettingsIntent::ShellChanged(value) => {
            state.set_shell(value);
            Task::none()
        },
        SettingsIntent::EditorChanged(value) => {
            state.set_editor(value);
            Task::none()
        },
        SettingsIntent::EqualizePanesToggled(value) => {
            state.set_equalize_panes(value);
            Task::none()
        },
        SettingsIntent::ThemeSelected(theme) => {
            state.set_theme(theme);
            Task::none()
        },
    }
}

/// Return whether the intent edits the draft the user is about to save.
fn changes_draft(event: &SettingsIntent) -> bool {
    matches!(
        event,
        SettingsIntent::Reset
            | SettingsIntent::LanguageChanged(_)
            | SettingsIntent::ShellChanged(_)
            | SettingsIntent::EditorChanged(_)
            | SettingsIntent::EqualizePanesToggled(_)
            | SettingsIntent::ThemeSelected(_)
    )
}

/// Start saving the draft and ask the route to write it; nothing happens when
/// a save is already running.
fn request_save_settings(state: &mut SettingsState) -> Task<SettingsEvent> {
    let Some(normalized) = state.begin_save() else {
        return Task::none();
    };

    Task::done(SettingsEvent::Effect(SettingsEffect::SaveRequested(
        normalized,
    )))
}

fn apply_loaded_settings(
    state: &mut SettingsState,
    load: SettingsLoad,
) -> SettingsData {
    let (settings, status) = load.into_parts();
    if let SettingsLoadStatus::Invalid(message) = &status {
        log::warn!("settings file invalid: {message}");
    }

    i18n::set_locale(settings.language().resolve());
    state.replace_with_settings(settings.clone());
    settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;
    use crate::testing::task_outputs::collect_outputs;
    use crate::theme::BuiltinTheme;
    use crate::widgets::settings::state::SettingsState;
    use crate::widgets::settings::types::{
        LanguageSetting, SettingsData, SettingsLoad, SettingsLoadStatus,
    };

    fn default_state() -> SettingsState {
        SettingsState::default()
    }

    fn effects(
        state: &mut SettingsState,
        intent: SettingsIntent,
    ) -> Vec<SettingsEffect> {
        collect_outputs(reduce(state, intent))
            .into_iter()
            .filter_map(|event| match event {
                SettingsEvent::Effect(effect) => Some(effect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reload_asks_the_route_to_read_the_settings() {
        let mut state = default_state();

        let effects = effects(&mut state, SettingsIntent::Reload);

        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::LoadRequested]
        ));
    }

    #[test]
    fn save_asks_the_route_to_write_the_settings_it_started_saving() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));

        let effects = effects(&mut state, SettingsIntent::Save);

        assert!(state.is_saving());
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::SaveRequested(settings)]
                if settings == state.draft()
        ));
    }

    #[test]
    fn a_second_save_while_one_runs_asks_for_nothing() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let _saved = state.begin_save().expect("save should start");

        let effects = effects(&mut state, SettingsIntent::Save);

        assert!(effects.is_empty());
    }

    #[test]
    fn given_save_completed_when_reduced_then_marks_state_saved() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        assert!(state.is_dirty());
        let normalized = state.begin_save().expect("save should start");

        let _task =
            reduce(&mut state, SettingsIntent::SaveCompleted(normalized));

        assert!(!state.is_dirty());
        assert!(!state.is_saving());
        assert_eq!(state.baseline(), state.draft());
    }

    #[test]
    fn given_save_in_flight_when_edit_intent_reduced_then_draft_unchanged() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let saved = state.begin_save().expect("save should start");

        let _task = reduce(
            &mut state,
            SettingsIntent::EditorChanged(String::from("nvim")),
        );

        assert_ne!(state.draft().terminal_editor(), "nvim");

        let _task =
            reduce(&mut state, SettingsIntent::SaveCompleted(saved.clone()));

        assert_eq!(state.baseline(), &saved);
        assert_eq!(state.draft(), &saved);
        assert!(!state.is_dirty());
    }

    #[test]
    fn given_save_in_flight_when_reset_intent_reduced_then_draft_unchanged() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let saved = state.begin_save().expect("save should start");

        let _task = reduce(&mut state, SettingsIntent::Reset);

        assert_eq!(state.draft(), &saved);
    }

    #[test]
    fn given_save_in_flight_when_language_changed_then_draft_unchanged() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let saved = state.begin_save().expect("save should start");

        let _task = reduce(
            &mut state,
            SettingsIntent::LanguageChanged(LanguageSetting::Fixed(
                Locale::ZhCn,
            )),
        );

        assert_eq!(state.draft(), &saved);
    }

    #[test]
    fn given_save_in_flight_when_save_intent_reduced_then_no_second_save() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let _saved = state.begin_save().expect("save should start");

        let _task = reduce(&mut state, SettingsIntent::Save);

        assert!(state.begin_save().is_none());
    }

    #[test]
    fn given_save_failed_when_reduced_then_keeps_state_dirty() {
        let mut state = default_state();
        state.set_editor(String::from("vim"));
        let _settings = state.begin_save().expect("save should start");
        assert!(state.is_dirty());

        let _task = reduce(
            &mut state,
            SettingsIntent::SaveFailed(String::from("write failed")),
        );

        assert!(state.is_dirty());
        assert!(!state.is_saving());
        assert_ne!(state.baseline(), state.draft());
        assert!(state.begin_save().is_some());
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

        let _task = reduce(&mut state, SettingsIntent::Reset);

        assert!(!state.is_dirty());
        assert_eq!(state.draft(), state.baseline());
    }

    #[test]
    fn given_shell_changed_when_reduced_then_draft_shell_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsIntent::ShellChanged(String::from("/usr/bin/fish")),
        );

        assert_eq!(state.draft().terminal_shell(), "/usr/bin/fish");
        assert!(state.is_dirty());
    }

    #[test]
    fn given_theme_selected_when_reduced_then_draft_theme_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight),
        );

        assert_eq!(state.draft().theme(), BuiltinTheme::OttyLight);
        assert!(state.is_dirty());
    }

    #[test]
    fn given_save_in_flight_when_theme_selected_then_draft_unchanged() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let saved = state.begin_save().expect("save should start");

        let _task = reduce(
            &mut state,
            SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight),
        );

        assert_eq!(state.draft(), &saved);
    }

    #[test]
    fn given_loaded_light_theme_when_reload_applied_then_draft_uses_it() {
        let mut state = default_state();
        let mut loaded = SettingsData::default();
        loaded.set_theme(BuiltinTheme::OttyLight);

        let applied = apply_loaded_settings(
            &mut state,
            SettingsLoad::new(loaded, SettingsLoadStatus::Loaded),
        );

        assert_eq!(applied.theme(), BuiltinTheme::OttyLight);
        assert_eq!(state.draft().theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn given_language_changed_when_reduced_then_draft_language_updated() {
        let mut state = default_state();

        let _task = reduce(
            &mut state,
            SettingsIntent::LanguageChanged(LanguageSetting::Fixed(
                Locale::ZhCn,
            )),
        );

        assert_eq!(
            state.draft().language(),
            LanguageSetting::Fixed(Locale::ZhCn)
        );
        assert!(state.is_dirty());
    }
}
