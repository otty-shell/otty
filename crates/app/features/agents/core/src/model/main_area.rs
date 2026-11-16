use std::path::PathBuf;
use std::time::SystemTime;

use otty_agents::{Profile, Project, SessionId};

use super::super::age::age_text;
use super::super::state::AgentsState;
use super::folder_name;
use super::profile_details::ProfileDetailsViewModel;

/// What fills the main area of Agents mode (spec Decision 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MainAreaViewModel {
    /// The profiles are read for the first time.
    Loading,
    /// No profile was found: no agent is installed.
    NoAgent,
    /// No project is selected: a card that says what the mode is for.
    Start,
    /// The selected project with its sessions.
    Project(ProjectViewModel),
    /// The details of the selected profile.
    Profile(ProfileDetailsViewModel),
}

impl MainAreaViewModel {
    /// Build the main area for the current state, with the ages of the
    /// sessions counted up to `now`.
    pub(in super::super) fn from_state(
        state: &AgentsState,
        now: SystemTime,
    ) -> Self {
        if !state.is_loaded() {
            return Self::Loading;
        }
        if state.profiles().is_empty() {
            return Self::NoAgent;
        }

        if let Some(shown) = state.selected_profile() {
            return Self::Profile(ProfileDetailsViewModel::from_profile(
                state, shown,
            ));
        }

        match state.selected_project() {
            Some(project) => Self::Project(ProjectViewModel::from_project(
                state, project, now,
            )),
            None => Self::Start,
        }
    }
}

/// The selected project: its folder, New session and its sessions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectViewModel {
    /// The project folder.
    pub(crate) folder: PathBuf,
    /// The last part of the folder path.
    pub(crate) name: String,
    /// The whole folder path.
    pub(crate) path: String,
    /// Whether the folder no longer exists.
    pub(crate) is_missing: bool,
    /// Whether New session can start: the folder exists and a profile can
    /// start a session.
    pub(crate) can_start: bool,
    /// The profiles to choose from while New session asks which one.
    pub(crate) choices: Option<Vec<ProfileChoiceViewModel>>,
    /// The sessions, newest first.
    pub(crate) sessions: Vec<SessionRowViewModel>,
}

impl ProjectViewModel {
    fn from_project(
        state: &AgentsState,
        project: &Project,
        now: SystemTime,
    ) -> Self {
        let folder = project.folder();
        let is_missing = state.is_missing(folder);
        let startable = state.startable_profiles();

        let choices = state.is_choosing_profile().then(|| {
            startable
                .iter()
                .map(|profile| ProfileChoiceViewModel {
                    icon: state.icon_of(profile.agent()),
                    profile: (*profile).clone(),
                })
                .collect()
        });
        let sessions = project
            .sessions()
            .iter()
            .map(|entry| {
                let profile = entry.profile();
                let session = entry.session();

                SessionRowViewModel {
                    id: session.id().clone(),
                    title: session.title().to_string(),
                    icon: state.icon_of(profile.agent()),
                    profile: profile.name().to_string(),
                    age: age_text(now, session.changed_at()),
                    branch: session.branch().map(str::to_string),
                    can_continue: !is_missing && state.can_start(profile),
                }
            })
            .collect();

        Self {
            folder: folder.to_path_buf(),
            name: folder_name(folder),
            path: folder.to_string_lossy().into_owned(),
            is_missing,
            can_start: !is_missing && !startable.is_empty(),
            choices,
            sessions,
        }
    }
}

/// One profile a new session can start with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileChoiceViewModel {
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The profile, sent when the choice is pressed.
    pub(crate) profile: Profile,
}

/// One session of the selected project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionRowViewModel {
    /// The session, sent with Continue.
    pub(crate) id: SessionId,
    /// The title of the session.
    pub(crate) title: String,
    /// The name of the agent's icon.
    pub(crate) icon: &'static str,
    /// The name of the profile the session belongs to.
    pub(crate) profile: String,
    /// How long ago the session last changed.
    pub(crate) age: String,
    /// The git branch the session ran on, when known.
    pub(crate) branch: Option<String>,
    /// Whether Continue can open the session.
    pub(crate) can_continue: bool,
}

#[cfg(test)]
mod tests {
    use otty_agents::ProfileStatus;
    use otty_app_i18n::{self as i18n, Key};

    use super::*;
    use crate::event::AgentsIntent;
    use crate::reducer::reduce;
    use crate::testing::{
        agents, claude, codex, connected, env, now, read, session,
        two_agents_with_sessions, with_profiles,
    };
    use crate::types::LoadedSessions;

    fn select(state: &mut AgentsState, folder: &str) {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
    }

    fn project(state: &AgentsState) -> ProjectViewModel {
        match MainAreaViewModel::from_state(state, now()) {
            MainAreaViewModel::Project(project) => project,
            other => panic!("expected a project, got {other:?}"),
        }
    }

    #[test]
    fn the_first_read_shows_that_it_loads() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Loading
        );
    }

    #[test]
    fn with_no_profile_the_card_says_no_agent_was_found() {
        let state = with_profiles(&[]);

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::NoAgent
        );
    }

    #[test]
    fn with_no_project_selected_the_card_says_what_the_mode_is_for() {
        let mut state = two_agents_with_sessions();

        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Start
        );

        select(&mut state, "/work/none");
        assert_eq!(
            MainAreaViewModel::from_state(&state, now()),
            MainAreaViewModel::Start
        );
    }

    #[test]
    fn a_selected_profile_shows_its_details() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let vm = MainAreaViewModel::from_state(&state, now());

        assert!(matches!(
            vm,
            MainAreaViewModel::Profile(details) if details.profile == codex()
        ));
    }

    #[test]
    fn selecting_a_project_shows_its_sessions_newest_first() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![
                session("c1", "/work/otty", 1_000_000 - 120)
                    .with_branch("main"),
            ],
        );
        read(
            &mut state,
            &codex(),
            vec![session("x1", "/work/otty", 1_000_000 - 30)],
        );
        select(&mut state, "/work/otty");

        let project = project(&state);

        assert_eq!(project.name, "otty");
        assert_eq!(project.path, "/work/otty");
        assert!(!project.is_missing);
        assert!(project.can_start);
        assert_eq!(project.choices, None);
        assert_eq!(
            project.sessions,
            vec![
                SessionRowViewModel {
                    id: SessionId::new("x1"),
                    title: String::from("Title of x1"),
                    icon: "codex",
                    profile: String::from("Codex"),
                    age: i18n::t(Key::AgentsAgeJustNow).to_string(),
                    branch: None,
                    can_continue: true,
                },
                SessionRowViewModel {
                    id: SessionId::new("c1"),
                    title: String::from("Title of c1"),
                    icon: "claude-code",
                    profile: String::from("Claude Code"),
                    age: i18n::with_count(Key::TplAgentsAgeMinutes, 2),
                    branch: Some(String::from("main")),
                    can_continue: true,
                },
            ]
        );
    }

    #[test]
    fn new_session_lists_the_profiles_that_can_start_while_it_asks() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), ProfileStatus::NotLoggedIn),
            (
                crate::testing::claude_work(),
                ProfileStatus::ProgramNotFound,
            ),
        ]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);
        select(&mut state, "/work/otty");
        let _ = reduce(&mut state, AgentsIntent::NewSession);

        let choices = project(&state).choices.expect("New session asks");

        assert_eq!(
            choices,
            vec![
                ProfileChoiceViewModel {
                    icon: "claude-code",
                    profile: claude(),
                },
                ProfileChoiceViewModel {
                    icon: "codex",
                    profile: codex(),
                },
            ]
        );
    }

    #[test]
    fn a_missing_project_offers_neither_continue_nor_new_session() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Ok(vec![session("c1", "/work/gone", 1)]),
                missing_folders: vec![PathBuf::from("/work/gone")],
            }),
        );
        select(&mut state, "/work/gone");

        let project = project(&state);

        assert!(project.is_missing);
        assert!(!project.can_start);
        assert!(!project.sessions[0].can_continue);
    }

    #[test]
    fn a_session_of_a_profile_whose_program_is_not_found_can_not_continue() {
        let mut state =
            with_profiles(&[(claude(), ProfileStatus::ProgramNotFound)]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);
        select(&mut state, "/work/otty");

        let project = project(&state);

        assert!(!project.can_start);
        assert!(!project.sessions[0].can_continue);
        assert_eq!(
            project.sessions[0].age,
            i18n::with_count(Key::TplAgentsAgeWeeks, 1)
        );
    }
}
