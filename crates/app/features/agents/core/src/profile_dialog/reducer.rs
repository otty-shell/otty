use std::path::{Path, PathBuf};

use otty_agents::{AgentId, Profile, proposed_config_dir};
use otty_app_i18n::{self as i18n, Key};

use super::admin_key_fields::AdminKeyFields;
use super::event::{FolderChoice, ProfileDialogIntent};
use super::state::ProfileDialog;
use crate::event::AgentsOutcome;
use crate::profile_changes::{self, ProfileEdit};
use crate::state::AgentsState;

/// Open the dialog that adds a profile, with the first agent OTTY knows.
pub(crate) fn open_add(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(agent) = state.agents().first() else {
        return Vec::new();
    };
    let agent = agent.entry().id().clone();

    state.open_dialog(ProfileDialog::add(agent));
    Vec::new()
}

/// Open the dialog that edits `profile`: a default profile too.
///
/// An API key profile whose agent has a bill gets the admin key field of
/// the organization it pays through, when one was found in its files.
pub(crate) fn open_edit(
    state: &mut AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    let profiles = state.profiles();
    let admin_key = profiles
        .get(profile)
        .filter(|shown| shown.is_api_key() && profiles.has_bill(shown))
        .and_then(|shown| {
            let organization = shown.organization()?.clone();
            let saved = profiles.admin_key_of(shown);
            Some(AdminKeyFields::new(organization, saved))
        });

    state.open_dialog(ProfileDialog::edit(profile, admin_key));
    Vec::new()
}

/// Reduce what the user does in the open profile dialog. The checks of the
/// name and the folder run as the user types; Add or Save does nothing
/// while one fails.
pub(crate) fn reduce(
    state: &mut AgentsState,
    intent: ProfileDialogIntent,
) -> Vec<AgentsOutcome> {
    let home = state.env().home().to_path_buf();
    let Some(dialog) = state.dialog_mut() else {
        return Vec::new();
    };

    match intent {
        ProfileDialogIntent::AgentListToggled => {
            dialog.toggle_agent_list();
            Vec::new()
        },
        ProfileDialogIntent::AgentPicked(agent) => {
            dialog.close_agent_list();
            pick_agent(state, agent);
            Vec::new()
        },
        ProfileDialogIntent::NameChanged(name) => {
            set_name(state, name);
            Vec::new()
        },
        ProfileDialogIntent::FolderChoicePicked(choice) => {
            dialog.pick_folder_choice(choice);
            Vec::new()
        },
        ProfileDialogIntent::Folder(intent) => {
            dialog.change_folder(intent, &home)
        },
        ProfileDialogIntent::AdminKeyChoicePicked(choice) => {
            if let Some(fields) = dialog.admin_key_fields_mut() {
                fields.pick_key_choice(choice);
            }
            Vec::new()
        },
        ProfileDialogIntent::AdminKeyChanged(text) => {
            if let Some(fields) = dialog.admin_key_fields_mut() {
                fields.set_key_text(text);
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
    if !is_adding || state.agent(&agent).is_none() {
        return;
    }

    let name = state
        .dialog()
        .map(|dialog| dialog.name().to_string())
        .unwrap_or_default();
    let proposal = proposal(state, &agent, &name);
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
            let saved = profile_changes::added(
                saved,
                dialog.agent().clone(),
                name,
                config_dir,
            );
            (saved, create_dir)
        },
        Some(editing) => {
            if state.agent(editing.agent()).is_none() {
                return Vec::new();
            }
            let edit = ProfileEdit {
                name,
                config_dir,
                admin_key: dialog.admin_key_edit(home),
            };
            let saved = profile_changes::edited(
                saved,
                state.agent_name(editing.agent()),
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
    agent: &AgentId,
    name: &str,
) -> Option<PathBuf> {
    let agent = state.agent(agent)?;

    proposed_config_dir(agent.config_folder(), state.env(), name)
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
    use crate::folder_field::FolderFieldIntent;
    use crate::profile_dialog::{AdminKeyChoice, FolderChoice, ProfileDialog};
    use crate::reducer::reduce as reduce_agents;
    use crate::testing::{
        admin_key, api_key, claude, claude_id, claude_work, codex, codex_id,
        connected, folder_looked_up, folder_query, organization,
        saved_with_admin_key, with_accounts, with_saved_profiles,
    };
    use crate::types::{FolderAsker, FolderQuery};

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
        let _ = send(state, typed(path));
    }

    /// `path` typed in the config folder field.
    fn typed(path: &str) -> ProfileDialogIntent {
        ProfileDialogIntent::Folder(FolderFieldIntent::Changed(path.into()))
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
        Profile::added(claude_id(), name, PathBuf::from(folder))
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
        assert_eq!(dialog.agent(), &claude_id());
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
            send(&mut state, ProfileDialogIntent::AgentPicked(codex_id()));

        assert!(outcomes.is_empty());
        let dialog = state.dialog().expect("the dialog is open");
        assert_eq!(dialog.agent(), &codex_id());
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
        let _ = send(&mut state, ProfileDialogIntent::AgentPicked(codex_id()));

        assert!(closed_at_first);
        assert!(opened);
        assert!(closed_again);
        assert!(!is_open(&state), "a pick closes the list");
        assert_eq!(state.dialog().map(ProfileDialog::agent), Some(&codex_id()));
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
                hidden_profiles: Vec::new(),
                saved: None,
                problem: Some(String::from("broken")),
                time_zone: None,
                read_at: std::time::SystemTime::UNIX_EPOCH,
                keyring_entries: Vec::new(),
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
                    DefaultProfileChanges::new(claude_id())
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
                SavedProfiles::default().with_default_changes(
                    DefaultProfileChanges::new(codex_id())
                ),
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
                    &claude_id(),
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
        folder_query(FolderAsker::ProfileDialog, folder, list_in, prefix)
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
            folder_looked_up(
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

        let outcomes = send(&mut state, typed("~/.c"));

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

        let outcomes = send(&mut state, typed("~/.c"));

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

        let outcomes = send(&mut state, typed("/data/"));

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

        let outcomes = send(
            &mut state,
            ProfileDialogIntent::Folder(FolderFieldIntent::SuggestionPicked(0)),
        );
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
            folder_looked_up(
                for_new_session,
                false,
                &["/home/user/.claude-work"],
            ),
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
            folder_looked_up(
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

    // The admin key of the organization an API key profile pays through.

    /// Spec 2026-11-19, Decision 7: the admin key is saved for the
    /// organization, keyed by its id, not in the profile.
    #[test]
    fn editing_an_api_key_profile_saves_the_admin_key_of_its_organization() {
        let saved = SavedProfiles::default().with_added(claude_work());
        let mut state = with_accounts(
            &[(claude_work(), api_key(), vec![organization()])],
            saved.clone(),
        );
        let _ =
            reduce_agents(&mut state, AgentsIntent::EditProfile(claude_work()));
        let _ = send(
            &mut state,
            ProfileDialogIntent::AdminKeyChanged(String::from(
                "~/.config/anthropic-admin-key",
            )),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        let key = AdminKeyRef::File(PathBuf::from(
            "/home/user/.config/anthropic-admin-key",
        ));
        assert_eq!(
            outcomes,
            save(
                saved
                    .with_admin_key_of(&organization(), Some(key))
                    .with_added_replaced(
                        &claude_id(),
                        "Claude Code · Work",
                        claude_work()
                    ),
                None
            )
        );
    }

    #[test]
    fn the_admin_key_of_a_default_profile_can_be_a_variable_or_removed() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
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
        let named = send(&mut state, ProfileDialogIntent::Submitted);
        let mut removing = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            saved_with_admin_key(),
        );
        let _ =
            reduce_agents(&mut removing, AgentsIntent::EditProfile(claude()));
        let _ = send(
            &mut removing,
            ProfileDialogIntent::AdminKeyChanged(String::new()),
        );

        let removed = send(&mut removing, ProfileDialogIntent::Submitted);

        let unchanged = SavedProfiles::default()
            .with_default_changes(DefaultProfileChanges::new(claude_id()));
        assert_eq!(
            named,
            save(
                unchanged
                    .clone()
                    .with_admin_key_of(&organization(), Some(admin_key())),
                None
            )
        );
        assert_eq!(removed, save(unchanged, None));
    }

    #[test]
    fn an_admin_key_path_that_is_not_full_blocks_save() {
        let mut state = with_accounts(
            &[(claude(), api_key(), vec![organization()])],
            SavedProfiles::default(),
        );
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(claude()));
        let _ = send(
            &mut state,
            ProfileDialogIntent::AdminKeyChanged(String::from("admin-key")),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert!(outcomes.is_empty());
        assert!(state.dialog().is_some_and(|dialog| !dialog.is_saving()));
    }

    /// An edit of a profile with no admin key field keeps the admin key
    /// of its organization as it was saved.
    #[test]
    fn a_subscription_has_no_admin_key_field_and_keeps_what_was_saved() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            saved_with_admin_key(),
        );
        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(claude()));
        let has_field = state
            .dialog()
            .map(|dialog| dialog.admin_key_fields().is_some());

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(has_field, Some(false));
        assert_eq!(
            outcomes,
            save(
                saved_with_admin_key().with_default_changes(
                    DefaultProfileChanges::new(claude_id())
                ),
                None
            )
        );
    }

    #[test]
    fn a_codex_api_key_profile_has_no_admin_key_field() {
        let mut state = with_saved_profiles(
            &[(codex(), api_key())],
            SavedProfiles::default(),
        );

        let _ = reduce_agents(&mut state, AgentsIntent::EditProfile(codex()));

        let fields = state.dialog().and_then(ProfileDialog::admin_key_fields);
        assert!(fields.is_none());
    }
}
