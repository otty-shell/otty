#![forbid(clippy::disallowed_methods)]

use super::event::{SettingsIntent, SettingsOutcome};
use super::problem::SettingsProblem;
use super::state::SettingsState;
use super::types::{SettingsLoad, SettingsLoadStatus};

/// Reduce a settings intent into state changes and the outcomes the frame
/// reads: reading and writing the file, applying saved settings, the
/// keyboard and failures to tell.
pub(super) fn reduce(
    state: &mut SettingsState,
    intent: SettingsIntent,
) -> Vec<SettingsOutcome> {
    if state.is_saving() && changes_draft(&intent) {
        return Vec::new();
    }

    match intent {
        SettingsIntent::Open => open(state),
        SettingsIntent::Close => close(state),
        SettingsIntent::ReloadLoaded(load) => reload_loaded(state, load),
        SettingsIntent::ReloadFailed(message) => {
            log::warn!("settings read failed: {message}");
            vec![SettingsOutcome::Failed(SettingsProblem::ReadFailed(
                message,
            ))]
        },
        SettingsIntent::Save => request_save_settings(state),
        SettingsIntent::SaveCompleted(settings) => {
            state.mark_saved(settings.clone());
            vec![SettingsOutcome::Applied(settings)]
        },
        SettingsIntent::SaveFailed(message) => {
            state.mark_save_failed();
            log::warn!("settings save failed: {message}");
            vec![SettingsOutcome::Failed(SettingsProblem::SaveFailed(
                message,
            ))]
        },
        SettingsIntent::Reset => {
            state.reset();
            Vec::new()
        },
        SettingsIntent::LanguageChanged(value) => {
            state.set_language(value);
            Vec::new()
        },
        SettingsIntent::ShellChanged(value) => {
            state.set_shell(value);
            Vec::new()
        },
        SettingsIntent::EditorChanged(value) => {
            state.set_editor(value);
            Vec::new()
        },
        SettingsIntent::EqualizePanesToggled(value) => {
            state.set_equalize_panes(value);
            Vec::new()
        },
        SettingsIntent::ThemeSelected(theme) => {
            state.set_theme(theme);
            Vec::new()
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

/// Open the dialog: it takes the keyboard from every widget, the terminal
/// too, and the saved settings are read again. Nothing happens while it is
/// open, so the edits stay.
fn open(state: &mut SettingsState) -> Vec<SettingsOutcome> {
    if !state.open() {
        return Vec::new();
    }

    vec![SettingsOutcome::Opened, SettingsOutcome::ReadSaved]
}

/// Close the dialog and drop the unsaved edits, and report it, so the frame
/// gives the keyboard back. Nothing happens while it is closed.
fn close(state: &mut SettingsState) -> Vec<SettingsOutcome> {
    if !state.close() {
        return Vec::new();
    }

    vec![SettingsOutcome::Closed]
}

/// Start saving the draft and ask for it to be written; nothing happens when
/// a save is already running or nothing changed.
fn request_save_settings(state: &mut SettingsState) -> Vec<SettingsOutcome> {
    let Some(normalized) = state.begin_save() else {
        return Vec::new();
    };

    vec![SettingsOutcome::WriteSaved(normalized)]
}

/// Show the settings read when the dialog opened and ask for them to be
/// applied. A broken file gave the defaults, and the user is told so after
/// they are applied, in the language they bring.
fn reload_loaded(
    state: &mut SettingsState,
    load: SettingsLoad,
) -> Vec<SettingsOutcome> {
    let (settings, status) = load.into_parts();

    state.replace_with_settings(settings.clone());

    let applied = SettingsOutcome::Applied(settings);
    let SettingsLoadStatus::Invalid(message) = status else {
        return vec![applied];
    };

    log::warn!("settings file invalid: {message}");
    vec![
        applied,
        SettingsOutcome::Failed(SettingsProblem::FileInvalid(message)),
    ]
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::Locale;
    use otty_app_theme::theme::BuiltinTheme;

    use super::*;
    use crate::types::{LanguageSetting, SettingsData};

    fn default_state() -> SettingsState {
        SettingsState::default()
    }

    /// A dialog that the user opened and edited.
    fn open_state_with_an_edit() -> SettingsState {
        let mut state = default_state();
        let _ = state.open();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));

        state
    }

    /// A state whose edited draft is being saved, with what is saved.
    fn state_saving_an_edit() -> (SettingsState, SettingsData) {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));
        let saved = state.begin_save().expect("save should start");

        (state, saved)
    }

    fn loaded(
        settings: SettingsData,
        status: SettingsLoadStatus,
    ) -> SettingsIntent {
        SettingsIntent::ReloadLoaded(SettingsLoad::new(settings, status))
    }

    /// The dialog takes the keyboard from the terminal, so typed keys do
    /// not reach the shell and Esc reaches the frame, and the saved
    /// settings are read again.
    #[test]
    fn open_shows_the_dialog_takes_the_keyboard_and_asks_to_read_the_settings()
    {
        let mut state = default_state();

        let outcomes = reduce(&mut state, SettingsIntent::Open);

        assert!(state.is_open());
        assert_eq!(
            outcomes,
            vec![SettingsOutcome::Opened, SettingsOutcome::ReadSaved]
        );
    }

    /// Owner, Q3: opening it again while it is open neither reads the file
    /// again nor drops what the user typed.
    #[test]
    fn open_while_open_does_nothing_and_keeps_the_edits() {
        let mut state = open_state_with_an_edit();
        let edited = state.draft().clone();

        let outcomes = reduce(&mut state, SettingsIntent::Open);

        assert!(outcomes.is_empty());
        assert_eq!(state.draft(), &edited);
        assert!(state.is_dirty());
    }

    /// Owner, Q1: closing drops the edits that were not saved.
    #[test]
    fn close_drops_the_edits_and_reports_that_it_closed() {
        let mut state = open_state_with_an_edit();

        let outcomes = reduce(&mut state, SettingsIntent::Close);

        assert!(!state.is_open());
        assert!(!state.is_dirty());
        assert_eq!(state.draft(), state.baseline());
        assert_eq!(outcomes, vec![SettingsOutcome::Closed]);
    }

    #[test]
    fn close_while_closed_reports_nothing() {
        let mut state = default_state();

        let outcomes = reduce(&mut state, SettingsIntent::Close);

        assert!(outcomes.is_empty());
    }

    /// The dialog can close while its save runs; the save still lands and
    /// the next open shows what was written.
    #[test]
    fn close_during_a_save_closes_and_the_save_still_lands() {
        let mut state = open_state_with_an_edit();
        let saved = state.begin_save().expect("save should start");

        let closed = reduce(&mut state, SettingsIntent::Close);
        let landed =
            reduce(&mut state, SettingsIntent::SaveCompleted(saved.clone()));

        assert_eq!(closed, vec![SettingsOutcome::Closed]);
        assert_eq!(landed, vec![SettingsOutcome::Applied(saved.clone())]);
        assert!(!state.is_open());
        assert!(!state.is_saving());
        assert_eq!(state.baseline(), &saved);
        assert_eq!(state.draft(), &saved);
    }

    /// Owner, Q2: Save writes and applies, and the dialog stays open and
    /// clean.
    #[test]
    fn a_finished_save_keeps_the_dialog_open_and_clean_and_applies_it() {
        let mut state = open_state_with_an_edit();
        let saved = state.begin_save().expect("save should start");

        let outcomes =
            reduce(&mut state, SettingsIntent::SaveCompleted(saved.clone()));

        assert!(state.is_open());
        assert!(!state.is_dirty());
        assert_eq!(outcomes, vec![SettingsOutcome::Applied(saved)]);
    }

    #[test]
    fn save_asks_to_write_the_settings_it_started_saving() {
        let mut state = default_state();
        state.set_shell(format!("{}-changed", state.draft().terminal_shell()));

        let outcomes = reduce(&mut state, SettingsIntent::Save);

        assert!(state.is_saving());
        assert_eq!(
            outcomes,
            vec![SettingsOutcome::WriteSaved(state.draft().clone())]
        );
    }

    #[test]
    fn a_second_save_while_one_runs_asks_for_nothing() {
        let (mut state, _saved) = state_saving_an_edit();

        let outcomes = reduce(&mut state, SettingsIntent::Save);

        assert!(outcomes.is_empty());
        assert!(state.begin_save().is_none());
    }

    #[test]
    fn given_save_completed_when_reduced_then_marks_state_saved() {
        let (mut state, saved) = state_saving_an_edit();

        let _applied = reduce(&mut state, SettingsIntent::SaveCompleted(saved));

        assert!(!state.is_dirty());
        assert!(!state.is_saving());
        assert_eq!(state.baseline(), state.draft());
    }

    /// While a save runs, edits and Reset change nothing and report
    /// nothing; the save then lands as it was started.
    #[test]
    fn given_save_in_flight_when_the_draft_is_edited_then_nothing_changes() {
        let edits = [
            SettingsIntent::EditorChanged(String::from("nvim")),
            SettingsIntent::Reset,
            SettingsIntent::LanguageChanged(LanguageSetting::Fixed(
                Locale::ZhCn,
            )),
            SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight),
            SettingsIntent::EqualizePanesToggled(false),
            SettingsIntent::ShellChanged(String::from("/bin/fish")),
        ];

        for edit in edits {
            let (mut state, saved) = state_saving_an_edit();

            let outcomes = reduce(&mut state, edit.clone());
            let _applied = reduce(
                &mut state,
                SettingsIntent::SaveCompleted(saved.clone()),
            );

            assert!(outcomes.is_empty(), "{edit:?}");
            assert_eq!(state.baseline(), &saved, "{edit:?}");
            assert_eq!(state.draft(), &saved, "{edit:?}");
            assert!(!state.is_dirty(), "{edit:?}");
        }
    }

    /// A failed save unlocks editing, so it can be tried again, and tells
    /// the user why.
    #[test]
    fn a_failed_save_unlocks_editing_and_tells_the_user() {
        let mut state = open_state_with_an_edit();
        let _saved = state.begin_save().expect("save should start");

        let outcomes = reduce(
            &mut state,
            SettingsIntent::SaveFailed(String::from("disk full")),
        );

        assert_eq!(
            outcomes,
            vec![SettingsOutcome::Failed(SettingsProblem::SaveFailed(
                String::from("disk full")
            ))]
        );
        assert!(!state.is_saving());
        assert!(state.is_dirty());
        assert_ne!(state.baseline(), state.draft());
        assert!(state.begin_save().is_some());
    }

    /// The dialog opens anyway and keeps what it showed; the user is told
    /// that the file could not be read.
    #[test]
    fn a_failed_read_tells_the_user() {
        let mut state = open_state_with_an_edit();
        let shown = state.draft().clone();

        let outcomes = reduce(
            &mut state,
            SettingsIntent::ReloadFailed(String::from("no access")),
        );

        assert_eq!(
            outcomes,
            vec![SettingsOutcome::Failed(SettingsProblem::ReadFailed(
                String::from("no access")
            ))]
        );
        assert_eq!(state.draft(), &shown);
    }

    /// A broken file still gives the defaults, which are applied first, so
    /// the user is told in the language they bring.
    #[test]
    fn an_invalid_file_applies_the_defaults_and_tells_the_user() {
        let mut state = open_state_with_an_edit();

        let outcomes = reduce(
            &mut state,
            loaded(
                SettingsData::default(),
                SettingsLoadStatus::Invalid(String::from("expected value")),
            ),
        );

        assert_eq!(
            outcomes,
            vec![
                SettingsOutcome::Applied(SettingsData::default()),
                SettingsOutcome::Failed(SettingsProblem::FileInvalid(
                    String::from("expected value")
                )),
            ]
        );
        assert_eq!(state.draft(), &SettingsData::default());
    }

    /// A read file, or a missing one at first start, tells nothing.
    #[test]
    fn a_read_or_a_missing_file_only_applies_the_settings() {
        for status in [SettingsLoadStatus::Loaded, SettingsLoadStatus::Missing]
        {
            let mut state = open_state_with_an_edit();

            let outcomes =
                reduce(&mut state, loaded(SettingsData::default(), status));

            assert_eq!(
                outcomes,
                vec![SettingsOutcome::Applied(SettingsData::default())]
            );
        }
    }

    #[test]
    fn given_reload_event_when_load_succeeds_then_state_replaced() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/zsh"));
        let read = SettingsData::default();

        let _applied = reduce(
            &mut state,
            loaded(read.clone(), SettingsLoadStatus::Loaded),
        );

        assert_eq!(state.draft(), &read);
        assert!(!state.is_dirty());
    }

    #[test]
    fn given_loaded_light_theme_when_reload_applied_then_draft_uses_it() {
        let mut state = default_state();
        let mut read = SettingsData::default();
        read.set_theme(BuiltinTheme::OttyLight);

        let outcomes = reduce(
            &mut state,
            loaded(read.clone(), SettingsLoadStatus::Loaded),
        );

        assert_eq!(outcomes, vec![SettingsOutcome::Applied(read)]);
        assert_eq!(state.draft().theme(), BuiltinTheme::OttyLight);
    }

    #[test]
    fn given_reset_command_when_reduced_then_draft_matches_baseline() {
        let mut state = default_state();
        state.set_shell(String::from("/bin/fish"));
        state.set_editor(String::from("nvim"));
        assert!(state.is_dirty());

        let outcomes = reduce(&mut state, SettingsIntent::Reset);

        assert!(outcomes.is_empty());
        assert!(!state.is_dirty());
        assert_eq!(state.draft(), state.baseline());
    }

    /// An edit changes only the draft: nothing is written or applied
    /// before Save.
    #[test]
    fn each_edit_changes_the_draft_and_reports_nothing() {
        let mut state = default_state();

        let outcomes = [
            reduce(
                &mut state,
                SettingsIntent::ShellChanged(String::from("/usr/bin/fish")),
            ),
            reduce(
                &mut state,
                SettingsIntent::EditorChanged(String::from("nvim")),
            ),
            reduce(
                &mut state,
                SettingsIntent::ThemeSelected(BuiltinTheme::OttyLight),
            ),
            reduce(
                &mut state,
                SettingsIntent::LanguageChanged(LanguageSetting::Fixed(
                    Locale::ZhCn,
                )),
            ),
            reduce(&mut state, SettingsIntent::EqualizePanesToggled(false)),
        ];

        let draft = state.draft();
        assert!(outcomes.iter().all(Vec::is_empty));
        assert_eq!(draft.terminal_shell(), "/usr/bin/fish");
        assert_eq!(draft.terminal_editor(), "nvim");
        assert_eq!(draft.theme(), BuiltinTheme::OttyLight);
        assert_eq!(draft.language(), LanguageSetting::Fixed(Locale::ZhCn));
        assert!(!draft.equalize_panes());
        assert!(state.is_dirty());
    }
}
