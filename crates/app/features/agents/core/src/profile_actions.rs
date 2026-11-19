//! What the buttons of a profile do: log it in and remove it, and the
//! answer of a save of `agents.json`.

use otty_agents::{Profile, SavedProfiles};
use otty_app_i18n::{self as i18n, Key};

use super::event::AgentsOutcome;
use super::profile_changes;
use super::rename_dialog::{self, RenameDialog};
use super::state::AgentsState;

/// The terminal tab that logs `profile` in, with the command of its agent.
/// A profile that is not shown or whose program is not found gives none.
pub(crate) fn log_in(
    state: &AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    if !state.profiles().can_start(profile) {
        return Vec::new();
    }
    let Some(agent) = state.agent(profile.agent()) else {
        return Vec::new();
    };

    vec![AgentsOutcome::OpenTab {
        title: profile.name().to_string(),
        folder: state.env().home().to_path_buf(),
        command: agent.login(profile),
    }]
}

/// Ask to confirm the removal of an added `profile`; a default profile or
/// one OTTY found can not be removed.
pub(crate) fn ask_removal(state: &mut AgentsState, profile: Profile) {
    if profile.is_default() || profile.is_found() {
        return;
    }

    state.ask_removal(profile);
}

/// Remove the profile the user confirmed from `agents.json`. Its config
/// folder and its sessions stay on disk.
pub(crate) fn confirm_removal(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(profile) = state.take_removing() else {
        return Vec::new();
    };
    let Some(saved) = state.profiles().saved() else {
        return vec![unread_file()];
    };

    let saved = profile_changes::removed(saved, &profile);
    vec![save(saved)]
}

/// Take the answer of a save of `agents.json`. The profile dialog or the
/// Rename dialog that waited for it closes and its profile is selected; a
/// failure stays in the dialog, or is told when no dialog waited. After a
/// save the profiles are read again.
pub(crate) fn saved(
    state: &mut AgentsState,
    result: Result<(), String>,
) -> Vec<AgentsOutcome> {
    let is_dialog_saving =
        state.dialog().is_some_and(|dialog| dialog.is_saving());

    let is_rename_saving = state.rename().is_some_and(RenameDialog::is_saving);

    if let Err(error) = result {
        let text = i18n::with_error(Key::TplAgentsSaveFailed, &error);
        if is_dialog_saving && let Some(dialog) = state.dialog_mut() {
            dialog.fail(text);
            return Vec::new();
        }
        if is_rename_saving && let Some(dialog) = state.rename_mut() {
            dialog.fail(text);
            return Vec::new();
        }
        return vec![AgentsOutcome::Failed(text)];
    }

    if is_dialog_saving && let Some(dialog) = state.take_dialog() {
        state.select_named(dialog.agent(), dialog.name().trim());
    }
    rename_dialog::close_after_save(state);

    if !state.profiles_mut().start_reload() {
        return Vec::new();
    }
    vec![AgentsOutcome::LoadProfiles]
}

/// The outcome that writes `saved` to `agents.json`.
pub(crate) fn save(saved: SavedProfiles) -> AgentsOutcome {
    AgentsOutcome::SaveProfiles {
        saved,
        create_dir: None,
    }
}

/// The notice that a change is not saved: `agents.json` could not be read.
pub(crate) fn unread_file() -> AgentsOutcome {
    AgentsOutcome::Failed(i18n::t(Key::AgentsSavedFileUnread).to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{AgentCommand, ProfileStatus, SavedProfiles};
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        claude, claude_work, codex, codex_work, connected, open_tab,
        profiles_loaded_from, with_saved_profiles,
    };

    fn save(saved: SavedProfiles) -> Vec<AgentsOutcome> {
        vec![AgentsOutcome::SaveProfiles {
            saved,
            create_dir: None,
        }]
    }

    #[test]
    fn log_in_runs_the_login_of_a_default_and_an_added_profile() {
        let mut state = with_saved_profiles(
            &[
                (claude(), ProfileStatus::NotLoggedIn),
                (codex_work(), ProfileStatus::NotLoggedIn),
            ],
            SavedProfiles::default().with_added(codex_work()),
        );

        let default = reduce(&mut state, AgentsIntent::LogIn(claude()));
        let added = reduce(&mut state, AgentsIntent::LogIn(codex_work()));

        assert_eq!(
            default,
            vec![open_tab(
                "Claude Code",
                "/home/user",
                AgentCommand::new("claude")
            )]
        );
        assert_eq!(
            added,
            vec![open_tab(
                "Codex · Work",
                "/home/user",
                AgentCommand::new("codex")
                    .with_arg("login")
                    .with_env("CODEX_HOME", "/home/user/.codex-work"),
            )]
        );
    }

    #[test]
    fn a_profile_whose_program_is_not_found_can_not_log_in() {
        let mut state = with_saved_profiles(
            &[(claude(), ProfileStatus::ProgramNotFound)],
            SavedProfiles::default(),
        );

        let outcomes = reduce(&mut state, AgentsIntent::LogIn(claude()));
        let unknown = reduce(&mut state, AgentsIntent::LogIn(codex()));

        assert!(outcomes.is_empty());
        assert!(unknown.is_empty());
    }

    #[test]
    fn a_saved_change_reads_the_profiles_again_after_the_read_on_its_way() {
        let mut state = with_saved_profiles(
            &[(claude_work(), connected())],
            SavedProfiles::default().with_added(claude_work()),
        );
        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude_work()));
        let _ = reduce(&mut state, AgentsIntent::RemoveConfirmed);
        let _ = reduce(&mut state, AgentsIntent::Refresh);

        let saved = reduce(&mut state, AgentsIntent::ProfilesSaved(Ok(())));
        let loaded = reduce(
            &mut state,
            profiles_loaded_from(&[], SavedProfiles::default()),
        );

        assert!(saved.is_empty(), "a read is on its way");
        assert_eq!(loaded, vec![AgentsOutcome::LoadProfiles]);
    }

    #[test]
    fn a_found_profile_can_not_be_removed() {
        let found = otty_agents::Profile::found(
            claude().agent(),
            "Work",
            PathBuf::from("/home/user/.claude-work"),
        );
        let mut state = with_saved_profiles(
            &[(found.clone(), connected())],
            SavedProfiles::default(),
        );

        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(found));

        assert_eq!(state.removing(), None);
    }

    #[test]
    fn a_failed_save_without_the_dialog_is_told() {
        let mut state = with_saved_profiles(
            &[(claude(), connected())],
            SavedProfiles::default(),
        );

        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfilesSaved(Err(String::from("disk full"))),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::Failed(i18n::with_error(
                Key::TplAgentsSaveFailed,
                "disk full"
            ))]
        );
    }

    #[test]
    fn removing_asks_first_and_only_drops_the_entry() {
        let saved = SavedProfiles::default()
            .with_added(claude_work())
            .with_added(codex_work());
        let mut state = with_saved_profiles(
            &[(claude_work(), connected()), (codex_work(), connected())],
            saved,
        );

        let asked =
            reduce(&mut state, AgentsIntent::RemoveProfile(codex_work()));
        let asking = state.removing().cloned();
        let removed = reduce(&mut state, AgentsIntent::RemoveConfirmed);

        assert!(asked.is_empty());
        assert_eq!(asking, Some(codex_work()));
        assert_eq!(
            removed,
            save(SavedProfiles::default().with_added(claude_work()))
        );
        assert_eq!(state.removing(), None);
    }

    #[test]
    fn a_removal_can_be_cancelled_and_a_default_profile_is_never_removed() {
        let mut state = with_saved_profiles(
            &[(claude(), connected()), (claude_work(), connected())],
            SavedProfiles::default().with_added(claude_work()),
        );

        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude_work()));
        let _ = reduce(&mut state, AgentsIntent::RemoveCancelled);
        let after_cancel = reduce(&mut state, AgentsIntent::RemoveConfirmed);
        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude()));

        assert!(after_cancel.is_empty());
        assert_eq!(state.removing(), None);
    }

    #[test]
    fn a_selected_profile_shows_until_a_project_is_selected() {
        let mut state = with_saved_profiles(
            &[(claude(), connected()), (codex(), connected())],
            SavedProfiles::default(),
        );

        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));
        let selected = state
            .selected_profile()
            .map(|shown| shown.profile().clone());
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/work/otty")),
        );

        assert_eq!(selected, Some(codex()));
        assert!(state.selected_profile().is_none());
        assert_eq!(
            state.selected_folder(),
            Some(std::path::Path::new("/work/otty"))
        );
    }
}
