use std::path::{Path, PathBuf};

use otty_agents::{AgentId, Profile, proposed_config_dir};
use otty_app_i18n::{self as i18n, Key};

use super::event::{FolderChoice, ProfileDialogIntent};
use super::money_fields::MoneyFields;
use super::state::ProfileDialog;
use crate::event::AgentsOutcome;
use crate::profile_changes::{self, ProfileEdit};
use crate::state::AgentsState;
use crate::types::LoadedFolder;

/// Open the dialog that adds a profile, with the first agent OTTY knows.
pub(crate) fn open_add(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(agent) = state.agents().first() else {
        return Vec::new();
    };
    let agent = agent.descriptor().id();

    state.open_dialog(ProfileDialog::add(agent));
    Vec::new()
}

/// Open the dialog that edits `profile`: a default profile too.
///
/// An API key profile gets its money fields: the budget, and the admin key
/// when its agent has a bill.
pub(crate) fn open_edit(
    state: &mut AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    let profiles = state.profiles();
    let money = profiles
        .is_api_key(profile)
        .then(|| MoneyFields::new(profile, profiles.has_bill(profile.agent())));

    state.open_dialog(ProfileDialog::edit(profile, money));
    Vec::new()
}

/// Reduce what the user does in the open profile dialog. The checks of the
/// name and the folder run as the user types; Add or Save does nothing
/// while one fails.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: ProfileDialogIntent,
) -> Vec<AgentsOutcome> {
    if state.dialog().is_none() {
        return Vec::new();
    }
    let home = state.env().home().to_path_buf();

    match intent {
        ProfileDialogIntent::AgentListToggled => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.toggle_agent_list();
            }
            Vec::new()
        },
        ProfileDialogIntent::AgentPicked(agent) => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.close_agent_list();
            }
            pick_agent(state, agent);
            Vec::new()
        },
        ProfileDialogIntent::NameChanged(name) => {
            set_name(state, name);
            Vec::new()
        },
        ProfileDialogIntent::FolderChoicePicked(choice) => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.pick_folder_choice(choice);
            }
            Vec::new()
        },
        ProfileDialogIntent::PathChanged(text) => {
            let Some(dialog) = state.dialog_mut() else {
                return Vec::new();
            };

            dialog.set_path(text, &home);
            dialog
                .typed_folder()
                .map(|typed| typed.look_up(&home))
                .unwrap_or_default()
        },
        ProfileDialogIntent::SuggestionPicked(index) => {
            let Some(typed) =
                state.dialog_mut().and_then(ProfileDialog::typed_folder_mut)
            else {
                return Vec::new();
            };
            if !typed.pick(index, &home) {
                return Vec::new();
            }

            typed.look_up(&home)
        },
        ProfileDialogIntent::SuggestionHighlighted(row) => {
            if let Some(typed) =
                state.dialog_mut().and_then(ProfileDialog::typed_folder_mut)
            {
                typed.highlight(row);
            }
            Vec::new()
        },
        ProfileDialogIntent::SuggestionsClosed => {
            if let Some(typed) =
                state.dialog_mut().and_then(ProfileDialog::typed_folder_mut)
            {
                typed.close();
            }
            Vec::new()
        },
        ProfileDialogIntent::AdminKeyChoicePicked(choice) => {
            if let Some(money) =
                state.dialog_mut().and_then(ProfileDialog::money_mut)
            {
                money.pick_key_choice(choice);
            }
            Vec::new()
        },
        ProfileDialogIntent::AdminKeyChanged(text) => {
            if let Some(money) =
                state.dialog_mut().and_then(ProfileDialog::money_mut)
            {
                money.set_key_text(text);
            }
            Vec::new()
        },
        ProfileDialogIntent::BudgetChanged(text) => {
            if let Some(money) =
                state.dialog_mut().and_then(ProfileDialog::money_mut)
            {
                money.set_budget(text);
            }
            Vec::new()
        },
        ProfileDialogIntent::Submitted => submit(state),
        ProfileDialogIntent::Cancelled => {
            let _ = state.take_dialog();
            Vec::new()
        },
    }
}

/// Take the answer about the typed config folder, while the dialog is open
/// and the text is still the one asked about. A folder that is not there
/// is no error of the dialog: a new profile may name a new folder.
pub(crate) fn apply_lookup(state: &mut AgentsState, loaded: LoadedFolder) {
    let home = state.env().home().to_path_buf();
    let Some(dialog) = state.dialog_mut() else {
        return;
    };

    if !dialog.apply_lookup(loaded, &home) {
        log::debug!(
            "the answer about a config folder typed before was dropped"
        );
    }
}

/// Take the typed name, and propose the new folder from it.
fn set_name(state: &mut AgentsState, name: String) {
    let Some(agent) = state.dialog().map(ProfileDialog::agent) else {
        return;
    };

    let proposal = proposal(state, agent, &name);
    if let Some(dialog) = state.dialog_mut() {
        dialog.set_name(name, proposal);
    }
}

/// Pick another agent for the new profile and propose its new folder.
fn pick_agent(state: &mut AgentsState, agent: AgentId) {
    let is_adding = state
        .dialog()
        .is_some_and(|dialog| dialog.editing().is_none());
    if !is_adding || state.agent(agent).is_none() {
        return;
    }

    let name = state
        .dialog()
        .map(|dialog| dialog.name().to_string())
        .unwrap_or_default();
    let proposal = proposal(state, agent, &name);
    if let Some(dialog) = state.dialog_mut() {
        dialog.pick_agent(agent, proposal);
    }
}

/// Press Add or Save: the new content of `agents.json` is saved, with the
/// new folder created first when asked. Nothing happens while a check
/// fails, and nothing is written over an `agents.json` that could not be
/// read.
fn submit(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let home = state.env().home();
    let Some(dialog) = state.dialog() else {
        return Vec::new();
    };
    let checks = dialog.checks(home, &state.profiles().all());
    let Ok(config_dir) = checks.folder().map(Path::to_path_buf) else {
        return Vec::new();
    };
    if !checks.can_submit() {
        return Vec::new();
    }

    let Some(saved) = state.profiles().saved() else {
        if let Some(dialog) = state.dialog_mut() {
            dialog.fail(i18n::t(Key::AgentsSavedFileUnread).to_string());
        }
        return Vec::new();
    };
    let name = dialog.name().trim().to_string();
    let (saved, create_dir) = match dialog.editing() {
        None => {
            let create_dir = (dialog.folder_choice() == FolderChoice::Create)
                .then(|| config_dir.clone());
            let profile = Profile::added(dialog.agent(), name, config_dir);
            (saved.clone().with_added(profile), create_dir)
        },
        Some(editing) => {
            let Some(agent) = state.agent(editing.agent()) else {
                return Vec::new();
            };
            let (admin_key, monthly_budget) = dialog.money_settings(home);
            let edit = ProfileEdit {
                name,
                config_dir,
                admin_key,
                monthly_budget,
            };
            let saved = profile_changes::edited(
                saved,
                agent.descriptor().name(),
                editing,
                edit,
            );
            (saved, None)
        },
    };

    if let Some(dialog) = state.dialog_mut() {
        dialog.start_saving();
    }
    vec![AgentsOutcome::SaveProfiles { saved, create_dir }]
}

/// The folder proposed for a new profile of `agent` named `name`.
fn proposal(
    state: &AgentsState,
    agent: AgentId,
    name: &str,
) -> Option<PathBuf> {
    let agent = state.agent(agent)?;

    proposed_config_dir(agent.descriptor(), state.env(), name)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use otty_agents::{
        AdminKeyRef, DefaultProfileChanges, ProfileStatus, SavedProfiles,
    };
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::profile_dialog::{AdminKeyChoice, FolderChoice, ProfileDialog};
    use crate::reducer::reduce as reduce_agents;
    use crate::testing::{
        CLAUDE, CODEX, api_key, claude, claude_work, codex, connected, dollars,
        with_saved_profiles,
    };
    use crate::types::{FolderAsker, FolderQuery, LoadedFolder};

    /// Agents mode with the default Claude Code and Codex profiles and an
    /// empty `agents.json`.
    fn two_defaults() -> AgentsState {
        with_saved_profiles(
            &[(claude(), connected()), (codex(), connected())],
            SavedProfiles::default(),
        )
    }

    fn send(
        state: &mut AgentsState,
        intent: ProfileDialogIntent,
    ) -> Vec<AgentsOutcome> {
        reduce_agents(state, AgentsIntent::Dialog(intent))
    }

    fn type_name(state: &mut AgentsState, name: &str) {
        let _ = send(state, ProfileDialogIntent::NameChanged(name.into()));
    }

    fn type_path(state: &mut AgentsState, path: &str) {
        let _ = send(state, ProfileDialogIntent::PathChanged(path.into()));
    }

    fn save(
        saved: SavedProfiles,
        create_dir: Option<&str>,
    ) -> Vec<AgentsOutcome> {
        vec![AgentsOutcome::SaveProfiles {
            saved,
            create_dir: create_dir.map(PathBuf::from),
        }]
    }

    fn claude_added(name: &str, folder: &str) -> Profile {
        Profile::added(CLAUDE, name, PathBuf::from(folder))
    }

    fn opened_add(state: &mut AgentsState) -> Vec<AgentsOutcome> {
        reduce_agents(state, AgentsIntent::OpenAddProfile)
    }

    /// Owner, issue 26: no folder is looked for on disk; the dialog opens
    /// with a new folder.
    #[test]
    fn adding_opens_the_dialog_with_a_new_folder_and_reads_nothing() {
        let mut state = two_defaults();

        let outcomes = opened_add(&mut state);

        assert!(outcomes.is_empty());
        let dialog = state.dialog().expect("the dialog is open");
        assert_eq!(dialog.agent(), CLAUDE);
        assert_eq!(dialog.editing(), None);
        assert_eq!(dialog.folder_choice(), FolderChoice::Create);
    }

    #[test]
    fn adding_with_a_typed_path_saves_the_full_path() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Claude Code · Data");
        let _ = send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        type_path(&mut state, " ~/data/claude ");

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Claude Code · Data",
                    "/home/user/data/claude"
                )),
                None
            )
        );
    }

    #[test]
    fn adding_with_a_new_folder_proposes_it_from_the_name_and_creates_it() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Semrush");
        let proposed =
            state.dialog().map(|dialog| dialog.path_text().to_string());

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(proposed.as_deref(), Some("/home/user/.claude-semrush"));
        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Semrush",
                    "/home/user/.claude-semrush"
                )),
                Some("/home/user/.claude-semrush")
            )
        );
    }

    #[test]
    fn a_typed_path_of_the_new_folder_is_kept_when_the_name_changes() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Semrush");
        type_path(&mut state, "/data/claude-semrush");

        type_name(&mut state, "Semrush Team");

        assert_eq!(
            state.dialog().map(ProfileDialog::path_text),
            Some("/data/claude-semrush")
        );
    }

    #[test]
    fn a_name_with_no_proposal_starts_the_new_folder_empty() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Работа");

        let blocked = send(&mut state, ProfileDialogIntent::Submitted);
        let path = state.dialog().map(|dialog| dialog.path_text().to_string());
        type_path(&mut state, "~/.claude-rabota");
        let added = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(path.as_deref(), Some(""));
        assert!(blocked.is_empty());
        assert_eq!(
            added,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Работа",
                    "/home/user/.claude-rabota"
                )),
                Some("/home/user/.claude-rabota")
            )
        );
    }

    #[test]
    fn the_checks_block_add() {
        let cases: [(&str, &str); 4] = [
            ("  ", "/home/user/.claude-x"),
            ("Codex", "/home/user/.claude-x"),
            ("Claude Code · X", "/home/user/.claude"),
            ("Claude Code · X", "work/.claude-x"),
        ];

        for (name, path) in cases {
            let mut state = two_defaults();
            let _ = opened_add(&mut state);
            let _ = send(
                &mut state,
                ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
            );
            type_name(&mut state, name);
            type_path(&mut state, path);

            let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

            assert!(outcomes.is_empty(), "{name} in {path}");
            assert!(state.dialog().is_some_and(|dialog| !dialog.is_saving()));
        }
    }

    #[test]
    fn picking_another_agent_proposes_its_folder() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Work");

        let outcomes =
            send(&mut state, ProfileDialogIntent::AgentPicked(CODEX));

        assert!(outcomes.is_empty());
        let dialog = state.dialog().expect("the dialog is open");
        assert_eq!(dialog.agent(), CODEX);
        assert_eq!(dialog.path_text(), "/home/user/.codex-work");
    }

    #[test]
    fn the_agent_list_opens_and_closes_and_a_pick_closes_it() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        let is_open = |state: &AgentsState| {
            state
                .dialog()
                .is_some_and(ProfileDialog::is_agent_list_open)
        };
        let closed_at_first = !is_open(&state);

        let _ = send(&mut state, ProfileDialogIntent::AgentListToggled);
        let opened = is_open(&state);
        let _ = send(&mut state, ProfileDialogIntent::AgentListToggled);
        let closed_again = !is_open(&state);
        let _ = send(&mut state, ProfileDialogIntent::AgentListToggled);
        let _ = send(&mut state, ProfileDialogIntent::AgentPicked(CODEX));

        assert!(closed_at_first);
        assert!(opened);
        assert!(closed_again);
        assert!(!is_open(&state), "a pick closes the list");
        assert_eq!(state.dialog().map(ProfileDialog::agent), Some(CODEX));
    }

    #[test]
    fn a_saved_profile_closes_the_dialog_is_selected_and_read() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Semrush");
        let _ = send(&mut state, ProfileDialogIntent::Submitted);

        let outcomes =
            reduce_agents(&mut state, AgentsIntent::ProfilesSaved(Ok(())));
        let _ = reduce_agents(
            &mut state,
            crate::testing::profiles_loaded(&[
                (claude(), connected()),
                (
                    claude_added("Semrush", "/home/user/.claude-semrush"),
                    ProfileStatus::NotLoggedIn,
                ),
            ]),
        );

        assert_eq!(outcomes, vec![AgentsOutcome::LoadProfiles]);
        assert!(state.dialog().is_none());
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile().name()),
            Some("Semrush")
        );
    }

    #[test]
    fn a_failed_save_keeps_the_dialog_open_with_the_error() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Semrush");
        let _ = send(&mut state, ProfileDialogIntent::Submitted);

        let outcomes = reduce_agents(
            &mut state,
            AgentsIntent::ProfilesSaved(Err(String::from("disk full"))),
        );

        assert!(outcomes.is_empty());
        let dialog = state.dialog().expect("the dialog stays open");
        assert_eq!(
            dialog.error(),
            Some(
                i18n::with_error(Key::TplAgentsSaveFailed, "disk full")
                    .as_str()
            )
        );
        assert!(!dialog.is_saving());
    }

    #[test]
    fn cancel_closes_the_dialog() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);

        let outcomes = send(&mut state, ProfileDialogIntent::Cancelled);

        assert!(outcomes.is_empty());
        assert!(state.dialog().is_none());
    }

    #[test]
    fn a_change_is_not_saved_over_an_agents_json_that_could_not_be_read() {
        let mut state = two_defaults();
        let _ = reduce_agents(&mut state, AgentsIntent::Refresh);
        let _ = reduce_agents(
            &mut state,
            AgentsIntent::ProfilesLoaded(crate::types::LoadedProfiles {
                profiles: Vec::new(),
                saved: None,
                problem: Some(String::from("broken")),
                time_zone: None,
                read_at: std::time::SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = opened_add(&mut state);
        type_name(&mut state, "Semrush");

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert!(outcomes.is_empty());
        assert_eq!(
            state.dialog().and_then(ProfileDialog::error),
            Some(i18n::t(Key::AgentsSavedFileUnread))
        );
    }

    #[test]
    fn editing_a_default_profile_saves_only_its_changes() {
        let mut state = two_defaults();
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(claude()));
        let opened = state.dialog().map(|dialog| {
            (dialog.name().to_string(), dialog.path_text().to_string())
        });
        type_name(&mut state, "Claude Code · Home");
        type_path(&mut state, "/data/claude");

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            opened,
            Some((
                String::from("Claude Code"),
                String::from("/home/user/.claude")
            ))
        );
        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_default_changes(
                    DefaultProfileChanges::new(CLAUDE)
                        .with_name("Claude Code · Home")
                        .with_config_dir(PathBuf::from("/data/claude"))
                ),
                None
            )
        );
    }

    #[test]
    fn editing_a_default_profile_without_changes_saves_no_change() {
        let mut state = two_defaults();
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(codex()));

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default()
                    .with_default_changes(DefaultProfileChanges::new(CODEX)),
                None
            )
        );
    }

    #[test]
    fn editing_an_added_profile_replaces_its_entry() {
        let saved = SavedProfiles::default().with_added(claude_work());
        let mut state =
            with_saved_profiles(&[(claude_work(), connected())], saved.clone());
        let _ =
            reduce_agents(&mut state, AgentsIntent::EditProfile(claude_work()));
        type_name(&mut state, "Claude Code · Office");

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                saved.with_added_replaced(
                    CLAUDE,
                    "Claude Code · Work",
                    claude_added(
                        "Claude Code · Office",
                        "/home/user/.claude-work"
                    )
                ),
                None
            )
        );
    }

    // The folder suggestions of the typed config folder (issue 38).

    fn profile_query(folder: &str, list_in: &str, prefix: &str) -> FolderQuery {
        FolderQuery {
            asker: FolderAsker::ProfileDialog,
            folder: PathBuf::from(folder),
            list_in: PathBuf::from(list_in),
            prefix: String::from(prefix),
        }
    }

    fn looked_up(
        query: FolderQuery,
        is_folder: bool,
        suggestions: &[&str],
    ) -> AgentsIntent {
        AgentsIntent::FolderLookedUp(LoadedFolder {
            query,
            is_folder,
            suggestions: suggestions.iter().map(PathBuf::from).collect(),
        })
    }

    /// The add dialog with the typed path picked.
    fn adding_a_typed_folder() -> AgentsState {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        let _ = send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        state
    }

    fn suggestions(state: &AgentsState) -> Vec<String> {
        state
            .dialog()
            .map(|dialog| dialog.suggestions(Path::new("/home/user")))
            .unwrap_or_default()
    }

    /// The add dialog with `~/.c` typed and the answer that lists the
    /// hidden config folders of the home folder.
    fn with_config_folders_suggested() -> AgentsState {
        let mut state = adding_a_typed_folder();
        type_path(&mut state, "~/.c");
        let _ = reduce_agents(
            &mut state,
            looked_up(
                profile_query("/home/user/.c", "/home/user", ".c"),
                false,
                &["/home/user/.claude-work", "/home/user/.codex-work"],
            ),
        );
        state
    }

    /// Owner, issue 38: config folders are hidden, so `~/.c` asks for the
    /// hidden folders of the home folder, for the profile dialog.
    #[test]
    fn typing_the_config_folder_looks_it_up() {
        let mut state = adding_a_typed_folder();

        let outcomes = send(
            &mut state,
            ProfileDialogIntent::PathChanged(String::from("~/.c")),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LookUpFolder(profile_query(
                "/home/user/.c",
                "/home/user",
                ".c",
            ))]
        );
    }

    /// The path of a new folder OTTY creates gets no suggestions.
    #[test]
    fn typing_the_path_of_a_new_folder_looks_nothing_up() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);

        let outcomes = send(
            &mut state,
            ProfileDialogIntent::PathChanged(String::from("~/.c")),
        );

        assert!(outcomes.is_empty());
    }

    #[test]
    fn the_folders_of_the_answer_are_suggested() {
        let state = with_config_folders_suggested();

        assert_eq!(
            suggestions(&state),
            vec![
                String::from("~/.claude-work/"),
                String::from("~/.codex-work/")
            ]
        );
    }

    /// The edit dialog has the typed path only, with its suggestions.
    #[test]
    fn the_edit_dialog_suggests_folders_too() {
        let mut state = two_defaults();
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(codex()));

        let outcomes = send(
            &mut state,
            ProfileDialogIntent::PathChanged(String::from("/data/")),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LookUpFolder(profile_query(
                "/data", "/data", "",
            ))]
        );
    }

    /// A pick puts the folder with a slash in the field and lists the next
    /// level; the saved folder has no slash at the end.
    #[test]
    fn a_picked_folder_fills_the_field_and_lists_the_next_level() {
        let mut state = with_config_folders_suggested();
        type_name(&mut state, "Claude Code · Data");

        let outcomes =
            send(&mut state, ProfileDialogIntent::SuggestionPicked(0));
        let saved = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            state.dialog().map(ProfileDialog::path_text),
            Some("~/.claude-work/")
        );
        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LookUpFolder(profile_query(
                "/home/user/.claude-work",
                "/home/user/.claude-work",
                "",
            ))]
        );
        assert_eq!(
            saved,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Claude Code · Data",
                    "/home/user/.claude-work"
                )),
                None
            )
        );
    }

    /// Issue 39: a row highlighted from the keyboard is kept, typing
    /// clears it, and while the answer about the new text is on its way
    /// the list shows the rows that still match.
    #[test]
    fn a_highlighted_row_is_kept_until_typing_and_the_list_stays() {
        let mut state = with_config_folders_suggested();
        let highlighted = |state: &AgentsState| {
            state
                .dialog()
                .and_then(ProfileDialog::typed_folder)
                .and_then(|typed| typed.highlighted())
        };

        let _ = send(
            &mut state,
            ProfileDialogIntent::SuggestionHighlighted(Some(1)),
        );
        let kept = highlighted(&state);
        type_path(&mut state, "~/.co");

        assert_eq!(kept, Some(1));
        assert_eq!(highlighted(&state), None);
        assert_eq!(suggestions(&state), vec![String::from("~/.codex-work/")]);
    }

    #[test]
    fn closing_the_list_hides_the_suggestions_until_the_next_typing() {
        let mut state = with_config_folders_suggested();

        let outcomes = send(&mut state, ProfileDialogIntent::SuggestionsClosed);

        assert!(outcomes.is_empty());
        assert!(suggestions(&state).is_empty());
        assert!(state.dialog().is_some(), "the dialog stays open");
    }

    #[test]
    fn an_answer_for_an_older_text_is_dropped() {
        let mut state = adding_a_typed_folder();
        type_path(&mut state, "~/.c");
        type_path(&mut state, "~/.cl");

        let _ = reduce_agents(
            &mut state,
            looked_up(
                profile_query("/home/user/.c", "/home/user", ".c"),
                false,
                &["/home/user/.claude-work", "/home/user/.codex-work"],
            ),
        );

        assert!(suggestions(&state).is_empty());
    }

    /// An answer the New session dialog asked for does not reach the
    /// profile dialog, and the other way round.
    #[test]
    fn an_answer_for_the_other_dialog_is_dropped() {
        let mut state = adding_a_typed_folder();
        type_path(&mut state, "~/.c");
        let for_new_session = FolderQuery {
            asker: FolderAsker::NewSessionDialog,
            ..profile_query("/home/user/.c", "/home/user", ".c")
        };

        let _ = reduce_agents(
            &mut state,
            looked_up(for_new_session, false, &["/home/user/.claude-work"]),
        );

        assert!(suggestions(&state).is_empty());
    }

    /// The config folder of a new profile may not exist yet: the answer
    /// that it is no folder is no error and does not block Add.
    #[test]
    fn a_missing_folder_is_no_error_of_the_profile_dialog() {
        let mut state = adding_a_typed_folder();
        type_name(&mut state, "Claude Code · New");
        type_path(&mut state, "~/.claude-new");
        let _ = reduce_agents(
            &mut state,
            looked_up(
                profile_query(
                    "/home/user/.claude-new",
                    "/home/user",
                    ".claude-new",
                ),
                false,
                &[],
            ),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Claude Code · New",
                    "/home/user/.claude-new"
                )),
                None
            )
        );
    }

    /// Picking the new folder closes the suggestions of the typed path.
    #[test]
    fn picking_the_new_folder_shows_no_suggestion() {
        let mut state = with_config_folders_suggested();

        let _ = send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Create),
        );
        let _ = send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );

        assert!(suggestions(&state).is_empty());
    }

    // The admin key and the budget of an API key profile.

    #[test]
    fn editing_an_api_key_profile_saves_where_its_admin_key_is_and_its_budget()
    {
        let saved = SavedProfiles::default().with_added(claude_work());
        let mut state =
            with_saved_profiles(&[(claude_work(), api_key())], saved.clone());
        let _ =
            reduce_agents(&mut state, AgentsIntent::EditProfile(claude_work()));
        let _ = send(
            &mut state,
            ProfileDialogIntent::AdminKeyChanged(String::from(
                "~/.config/anthropic-admin-key",
            )),
        );
        let _ = send(
            &mut state,
            ProfileDialogIntent::BudgetChanged(String::from("300")),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        let edited = claude_work()
            .with_admin_key(Some(AdminKeyRef::File(PathBuf::from(
                "/home/user/.config/anthropic-admin-key",
            ))))
            .with_monthly_budget(Some(dollars(300)));
        assert_eq!(
            outcomes,
            save(
                saved.with_added_replaced(CLAUDE, "Claude Code · Work", edited),
                None
            )
        );
    }

    #[test]
    fn the_admin_key_of_a_default_profile_can_be_a_variable() {
        let mut state = with_saved_profiles(
            &[(claude(), api_key())],
            SavedProfiles::default(),
        );
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(claude()));
        let _ = send(
            &mut state,
            ProfileDialogIntent::AdminKeyChoicePicked(AdminKeyChoice::Variable),
        );
        let _ = send(
            &mut state,
            ProfileDialogIntent::AdminKeyChanged(String::from(
                "ANTHROPIC_ADMIN_KEY",
            )),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_default_changes(
                    DefaultProfileChanges::new(CLAUDE).with_admin_key(Some(
                        AdminKeyRef::EnvVar(String::from(
                            "ANTHROPIC_ADMIN_KEY"
                        ))
                    ))
                ),
                None
            )
        );
    }

    #[test]
    fn a_budget_that_is_not_dollars_blocks_save() {
        let mut state = with_saved_profiles(
            &[(claude(), api_key())],
            SavedProfiles::default(),
        );
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(claude()));
        let _ = send(
            &mut state,
            ProfileDialogIntent::BudgetChanged(String::from("lots")),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert!(outcomes.is_empty());
        assert!(state.dialog().is_some_and(|dialog| !dialog.is_saving()));
    }

    #[test]
    fn a_subscription_has_no_money_fields_and_keeps_what_it_saved() {
        let budget = Some(dollars(50));
        let saved = SavedProfiles::default().with_default_changes(
            DefaultProfileChanges::new(CLAUDE).with_monthly_budget(budget),
        );
        let profile = claude().with_monthly_budget(budget);
        let mut state =
            with_saved_profiles(&[(profile.clone(), connected())], saved);
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(profile));
        let has_fields = state.dialog().map(|dialog| dialog.money().is_some());
        let _ = send(
            &mut state,
            ProfileDialogIntent::BudgetChanged(String::from("999")),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(has_fields, Some(false));
        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_default_changes(
                    DefaultProfileChanges::new(CLAUDE)
                        .with_monthly_budget(budget)
                ),
                None
            )
        );
    }

    #[test]
    fn a_codex_api_key_profile_has_a_budget_but_no_admin_key() {
        let mut state = with_saved_profiles(
            &[(codex(), api_key())],
            SavedProfiles::default(),
        );

        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(codex()));

        let fields = state.dialog().and_then(ProfileDialog::money);
        assert!(fields.is_some_and(|fields| !fields.has_admin_key()));
    }
}
