use std::path::PathBuf;

use otty_agents::{AgentId, Profile, proposed_config_dir};
use otty_app_i18n::{self as i18n, Key};

use super::event::{FolderChoice, ProfileDialogIntent};
use super::state::ProfileDialog;
use crate::event::AgentsOutcome;
use crate::profile_changes::{self, ProfileEdit};
use crate::state::AgentsState;

/// Open the dialog that adds a profile, with the first agent OTTY knows,
/// and ask for the folders found for it.
pub(crate) fn open_add(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(agent) = state.agents().first() else {
        return Vec::new();
    };
    let agent = agent.descriptor().id();

    state.open_dialog(ProfileDialog::add(agent));
    vec![load_candidates(state, agent)]
}

/// Open the dialog that edits `profile`: a default profile too.
pub(crate) fn open_edit(
    state: &mut AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    state.open_dialog(ProfileDialog::edit(profile));
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
            pick_agent(state, agent)
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
        ProfileDialogIntent::FoundFolderPicked(folder) => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.pick_found(folder);
            }
            Vec::new()
        },
        ProfileDialogIntent::PathChanged(text) => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.set_path(text);
            }
            Vec::new()
        },
        ProfileDialogIntent::ProgramChanged(text) => {
            if let Some(dialog) = state.dialog_mut() {
                dialog.set_program(text);
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

/// Take the folders found for `agent` into the open dialog.
pub(crate) fn take_candidates(
    state: &mut AgentsState,
    agent: AgentId,
    folders: Vec<PathBuf>,
) {
    if let Some(dialog) = state.dialog_mut() {
        dialog.take_found(agent, folders);
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

/// Pick another agent for the new profile and ask for its found folders.
fn pick_agent(state: &mut AgentsState, agent: AgentId) -> Vec<AgentsOutcome> {
    let is_adding = state
        .dialog()
        .is_some_and(|dialog| dialog.editing().is_none());
    if !is_adding || state.agent(agent).is_none() {
        return Vec::new();
    }

    let name = state
        .dialog()
        .map(|dialog| dialog.name().to_string())
        .unwrap_or_default();
    let proposal = proposal(state, agent, &name);
    if let Some(dialog) = state.dialog_mut() {
        dialog.pick_agent(agent, proposal);
    }

    vec![load_candidates(state, agent)]
}

/// Press Add or Save: the new content of `agents.json` is saved, with the
/// new folder created first when asked. Nothing happens while a check
/// fails, and nothing is written over an `agents.json` that could not be
/// read.
fn submit(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let profiles = state.all_profiles();
    let home = state.env().home().to_path_buf();
    let Some(dialog) = state.dialog() else {
        return Vec::new();
    };
    if !dialog.can_submit(&home, &profiles) {
        return Vec::new();
    }
    let Ok(config_dir) = dialog.folder(&home, &profiles) else {
        return Vec::new();
    };

    let Some(saved) = state.saved() else {
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
            (profile_changes::added(saved, profile), create_dir)
        },
        Some(editing) => {
            let Some(agent) = state.agent(editing.agent()) else {
                return Vec::new();
            };
            let edit = ProfileEdit {
                name,
                config_dir,
                program: dialog.program_path(&home),
            };
            let saved = profile_changes::edited(
                saved,
                agent.descriptor(),
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

/// The outcome that finds the config folders of `agent` in the home
/// folder that no profile uses yet.
fn load_candidates(state: &AgentsState, agent: AgentId) -> AgentsOutcome {
    AgentsOutcome::LoadCandidates {
        agent,
        profiles: state.all_profiles(),
    }
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
    use otty_agents::{DefaultProfileChanges, ProfileStatus, SavedProfiles};
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::profile_dialog::{FolderChoice, ProfileDialog};
    use crate::reducer::reduce as reduce_agents;
    use crate::testing::{
        CLAUDE, CODEX, claude, claude_work, codex, connected,
        with_saved_profiles,
    };

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

    fn found(state: &mut AgentsState, agent: AgentId, folders: &[&str]) {
        let _ = reduce_agents(
            state,
            AgentsIntent::CandidatesLoaded {
                agent,
                folders: folders.iter().map(PathBuf::from).collect(),
            },
        );
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

    #[test]
    fn adding_opens_the_dialog_and_asks_for_the_found_folders() {
        let mut state = two_defaults();

        let outcomes = opened_add(&mut state);

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LoadCandidates {
                agent: CLAUDE,
                profiles: vec![claude(), codex()],
            }]
        );
        let dialog = state.dialog().expect("the dialog is open");
        assert_eq!(dialog.agent(), CLAUDE);
        assert_eq!(dialog.editing(), None);
        assert_eq!(dialog.folder_choice(), FolderChoice::Create);
    }

    #[test]
    fn adding_with_a_found_folder_saves_the_profile() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        found(
            &mut state,
            CLAUDE,
            &["/home/user/.claude-home", "/home/user/.claude-work"],
        );
        type_name(&mut state, "Claude Code · Work");
        let _ = send(
            &mut state,
            ProfileDialogIntent::FoundFolderPicked(PathBuf::from(
                "/home/user/.claude-work",
            )),
        );

        let outcomes = send(&mut state, ProfileDialogIntent::Submitted);

        assert_eq!(
            outcomes,
            save(
                SavedProfiles::default().with_added(claude_added(
                    "Claude Code · Work",
                    "/home/user/.claude-work"
                )),
                None
            )
        );
        assert!(state.dialog().is_some_and(ProfileDialog::is_saving));
    }

    #[test]
    fn a_found_folder_is_picked_until_the_user_picks_another_way() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        found(&mut state, CLAUDE, &["/home/user/.claude-home"]);
        let auto = state.dialog().map(ProfileDialog::folder_choice);

        let _ = send(
            &mut state,
            ProfileDialogIntent::FolderChoicePicked(FolderChoice::Typed),
        );
        found(&mut state, CLAUDE, &["/home/user/.claude-home"]);

        assert_eq!(auto, Some(FolderChoice::Found));
        assert_eq!(
            state.dialog().map(ProfileDialog::folder_choice),
            Some(FolderChoice::Typed)
        );
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
    fn picking_another_agent_asks_for_its_folders_and_drops_the_old_ones() {
        let mut state = two_defaults();
        let _ = opened_add(&mut state);
        type_name(&mut state, "Work");

        let outcomes =
            send(&mut state, ProfileDialogIntent::AgentPicked(CODEX));
        found(&mut state, CLAUDE, &["/home/user/.claude-work"]);

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LoadCandidates {
                agent: CODEX,
                profiles: vec![claude(), codex()],
            }]
        );
        let dialog = state.dialog().expect("the dialog is open");
        assert!(dialog.found().is_empty());
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
        let _ = send(
            &mut state,
            ProfileDialogIntent::ProgramChanged(String::from(
                "/opt/claude/bin/claude",
            )),
        );

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
                        .with_program(PathBuf::from("/opt/claude/bin/claude"))
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
}
