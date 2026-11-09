#![forbid(clippy::disallowed_methods)]

use iced::Task;

use super::event::{SettingsEffect, SettingsEvent, SettingsIntent};
use super::state::SettingsState;
use super::types::{SettingsLoad, SettingsLoadStatus};
use crate::focus;
use crate::i18n::{self, Key};

/// Reduce a settings intent event into state updates and effect tasks.
pub(super) fn reduce(
    state: &mut SettingsState,
    intent: SettingsIntent,
) -> Task<SettingsEvent> {
    if state.is_saving() && changes_draft(&intent) {
        return Task::none();
    }

    match intent {
        SettingsIntent::Open => open(state),
        SettingsIntent::Close => close(state),
        SettingsIntent::ReloadLoaded(load) => reload_loaded(state, load),
        SettingsIntent::ReloadFailed(message) => {
            log::warn!("settings read failed: {message}");
            failed(Key::TplSettingsReadFailed, &message)
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
            failed(Key::TplSettingsSaveFailed, &message)
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

/// Open the popup: it takes the keyboard from every widget, the terminal
/// too, and asks the effects to read the saved settings again. Nothing
/// happens while it is open, so the edits stay.
fn open(state: &mut SettingsState) -> Task<SettingsEvent> {
    if !state.open() {
        return Task::none();
    }

    Task::batch([
        focus::nothing(),
        Task::done(SettingsEvent::Effect(SettingsEffect::LoadRequested)),
    ])
}

/// Close the popup and drop the unsaved edits, and report it, so the effects
/// give the keyboard back. Nothing happens while it is closed.
fn close(state: &mut SettingsState) -> Task<SettingsEvent> {
    if !state.close() {
        return Task::none();
    }

    Task::done(SettingsEvent::Effect(SettingsEffect::Closed))
}

/// Start saving the draft and ask the effects to write it; nothing happens
/// when a save is already running.
fn request_save_settings(state: &mut SettingsState) -> Task<SettingsEvent> {
    let Some(normalized) = state.begin_save() else {
        return Task::none();
    };

    Task::done(SettingsEvent::Effect(SettingsEffect::SaveRequested(
        normalized,
    )))
}

/// Show the settings read when the popup opened and ask the effects to apply
/// them. A broken file gave the defaults, and the user is told so.
fn reload_loaded(
    state: &mut SettingsState,
    load: SettingsLoad,
) -> Task<SettingsEvent> {
    let (settings, status) = load.into_parts();

    i18n::set_locale(settings.language().resolve());
    state.replace_with_settings(settings.clone());

    let apply =
        Task::done(SettingsEvent::Effect(SettingsEffect::ApplyTheme(settings)));
    let SettingsLoadStatus::Invalid(message) = status else {
        return apply;
    };

    log::warn!("settings file invalid: {message}");
    Task::batch([apply, failed(Key::TplSettingsFileInvalid, &message)])
}

/// Tell the user about a failure: the `{error}` template behind `key`
/// filled with `error`.
fn failed(key: Key, error: &str) -> Task<SettingsEvent> {
    Task::done(SettingsEvent::Effect(SettingsEffect::Failed(
        i18n::with_error(key, error),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;
    use crate::testing::task_outputs::{collect_outputs, focuses_nothing};
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

    /// A popup that the user opened and edited.
    fn open_state_with_an_edit() -> SettingsState {
        let mut state = default_state();
        let _ = state.open();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));

        state
    }

    #[test]
    fn open_shows_the_popup_and_asks_the_route_to_read_the_settings() {
        let mut state = default_state();

        let effects = effects(&mut state, SettingsIntent::Open);

        assert!(state.is_open());
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::LoadRequested]
        ));
    }

    /// The popup takes the keyboard from the terminal, so typed keys do
    /// not reach the shell and Esc reaches the app.
    #[test]
    fn open_takes_the_keyboard_from_every_widget() {
        let mut state = default_state();

        let task = reduce(&mut state, SettingsIntent::Open);

        assert!(focuses_nothing(task));
    }

    /// Owner, Q3: opening it again while it is open neither reads the file
    /// again nor drops what the user typed.
    #[test]
    fn open_while_open_does_nothing_and_keeps_the_edits() {
        let mut state = open_state_with_an_edit();
        let edited = state.draft().clone();

        let effects = effects(&mut state, SettingsIntent::Open);
        let task = reduce(&mut state, SettingsIntent::Open);

        assert!(effects.is_empty());
        assert!(!focuses_nothing(task));
        assert_eq!(state.draft(), &edited);
        assert!(state.is_dirty());
    }

    /// Owner, Q1: closing drops the edits that were not saved.
    #[test]
    fn close_drops_the_edits_and_reports_that_it_closed() {
        let mut state = open_state_with_an_edit();

        let effects = effects(&mut state, SettingsIntent::Close);

        assert!(!state.is_open());
        assert!(!state.is_dirty());
        assert_eq!(state.draft(), state.baseline());
        assert!(matches!(effects.as_slice(), [SettingsEffect::Closed]));
    }

    #[test]
    fn close_while_closed_reports_nothing() {
        let mut state = default_state();

        let effects = effects(&mut state, SettingsIntent::Close);

        assert!(effects.is_empty());
    }

    /// The popup can close while its save runs; the save still lands and
    /// the next open shows what was written.
    #[test]
    fn close_during_a_save_closes_and_the_save_still_lands() {
        let mut state = open_state_with_an_edit();
        let saved = state.begin_save().expect("save should start");

        let effects = effects(&mut state, SettingsIntent::Close);
        let _task =
            reduce(&mut state, SettingsIntent::SaveCompleted(saved.clone()));

        assert!(matches!(effects.as_slice(), [SettingsEffect::Closed]));
        assert!(!state.is_open());
        assert!(!state.is_saving());
        assert_eq!(state.baseline(), &saved);
        assert_eq!(state.draft(), &saved);
    }

    /// Owner, Q2: Save writes and applies, and the popup stays open and
    /// clean.
    #[test]
    fn a_finished_save_keeps_the_popup_open_and_clean() {
        let mut state = open_state_with_an_edit();
        let saved = state.begin_save().expect("save should start");

        let effects =
            effects(&mut state, SettingsIntent::SaveCompleted(saved.clone()));

        assert!(state.is_open());
        assert!(!state.is_dirty());
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::ApplyTheme(applied)] if applied == &saved
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

    /// A failed save unlocks editing, as before, and tells the user why.
    #[test]
    fn a_failed_save_unlocks_editing_and_tells_the_user() {
        let mut state = open_state_with_an_edit();
        let _saved = state.begin_save().expect("save should start");

        let effects = effects(
            &mut state,
            SettingsIntent::SaveFailed(String::from("disk full")),
        );

        let expected =
            i18n::with_error(Key::TplSettingsSaveFailed, "disk full");
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::Failed(text)] if *text == expected
        ));
        assert!(!state.is_saving());
        assert!(state.is_dirty());
    }

    /// The popup opens anyway and keeps what it showed; the user is told
    /// that the file could not be read.
    #[test]
    fn a_failed_read_tells_the_user() {
        let mut state = open_state_with_an_edit();
        let shown = state.draft().clone();

        let effects = effects(
            &mut state,
            SettingsIntent::ReloadFailed(String::from("no access")),
        );

        let expected =
            i18n::with_error(Key::TplSettingsReadFailed, "no access");
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::Failed(text)] if *text == expected
        ));
        assert_eq!(state.draft(), &shown);
    }

    /// A broken file still gives the defaults, which are applied, and the
    /// user is told that the file was not used.
    #[test]
    fn an_invalid_file_applies_the_defaults_and_tells_the_user() {
        let mut state = open_state_with_an_edit();

        let effects = effects(
            &mut state,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                SettingsData::default(),
                SettingsLoadStatus::Invalid(String::from("expected value")),
            )),
        );

        let expected =
            i18n::with_error(Key::TplSettingsFileInvalid, "expected value");
        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::ApplyTheme(applied), SettingsEffect::Failed(text)]
                if *applied == SettingsData::default() && *text == expected
        ));
        assert_eq!(state.draft(), &SettingsData::default());
    }

    /// A read file, or a missing one at first start, tells nothing.
    #[test]
    fn a_read_or_a_missing_file_only_applies_the_settings() {
        for status in [SettingsLoadStatus::Loaded, SettingsLoadStatus::Missing]
        {
            let mut state = open_state_with_an_edit();

            let effects = effects(
                &mut state,
                SettingsIntent::ReloadLoaded(SettingsLoad::new(
                    SettingsData::default(),
                    status,
                )),
            );

            assert!(matches!(
                effects.as_slice(),
                [SettingsEffect::ApplyTheme(_)]
            ));
        }
    }

    #[test]
    fn given_reload_event_when_load_succeeds_then_state_replaced() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/zsh"));
        let loaded = SettingsData::default();

        let _task = reduce(
            &mut state,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                loaded.clone(),
                SettingsLoadStatus::Loaded,
            )),
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

        let effects = effects(
            &mut state,
            SettingsIntent::ReloadLoaded(SettingsLoad::new(
                loaded,
                SettingsLoadStatus::Loaded,
            )),
        );

        assert!(matches!(
            effects.as_slice(),
            [SettingsEffect::ApplyTheme(applied)]
                if applied.theme() == BuiltinTheme::OttyLight
        ));
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
