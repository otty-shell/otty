use std::path::Path;

use otty_agents::{AgentSession, Profile, SessionId};
use otty_app_domain::mode::Mode;

use super::event::{AgentsIntent, AgentsOutcome};
use super::state::AgentsState;
use super::{profile_actions, profile_dialog};

/// Reduce an Agents intent into state changes and the outcomes the binary
/// reads. The reducer reads no file and starts no program: it asks for the
/// reads and for the terminal tab with outcomes.
pub(super) fn reduce(
    state: &mut AgentsState,
    intent: AgentsIntent,
) -> Vec<AgentsOutcome> {
    match intent {
        AgentsIntent::ModeChanged(Mode::Agents) => {
            reduce(state, AgentsIntent::Enter)
        },
        AgentsIntent::ModeChanged(_) => reduce(state, AgentsIntent::Leave),
        AgentsIntent::Enter => {
            state.set_active(true);
            load_profiles(state)
        },
        AgentsIntent::Leave => {
            state.set_active(false);
            Vec::new()
        },
        AgentsIntent::Refresh => load_profiles(state),
        AgentsIntent::ProfilesLoaded(loaded) => {
            state.set_saved(loaded.saved);
            let to_read = state.apply_profiles(loaded.profiles);

            let mut outcomes: Vec<AgentsOutcome> = loaded
                .problem
                .map(AgentsOutcome::Failed)
                .into_iter()
                .collect();
            outcomes
                .extend(to_read.into_iter().map(AgentsOutcome::LoadSessions));
            outcomes.extend(load_limits(state));
            outcomes.extend(load_spend(state));
            if state.take_reload_wanted() {
                outcomes.extend(load_profiles(state));
            }
            outcomes
        },
        AgentsIntent::SessionsLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_sessions(loaded) {
                log::debug!(
                    "agent sessions of a profile no longer shown dropped: {name}"
                );
            }
            Vec::new()
        },
        AgentsIntent::LimitsLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_limits(loaded) {
                log::debug!(
                    "agent limits of a profile no longer shown dropped: {name}"
                );
            }
            Vec::new()
        },
        AgentsIntent::SpendLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_spend(loaded) {
                log::debug!(
                    "agent tokens of a profile no longer shown dropped: {name}"
                );
            }
            Vec::new()
        },
        AgentsIntent::LimitsTick if state.is_active() => load_limits(state),
        AgentsIntent::LimitsTick => Vec::new(),
        AgentsIntent::SelectProject(folder) => {
            state.select_project(folder);
            Vec::new()
        },
        AgentsIntent::Continue { folder, session } => {
            continue_session(state, &folder, &session)
        },
        AgentsIntent::NewSession => new_session(state),
        AgentsIntent::NewSessionWith(profile) => {
            state.set_choosing_profile(false);
            new_session_with(state, &profile)
        },
        AgentsIntent::CancelNewSession => {
            state.set_choosing_profile(false);
            Vec::new()
        },
        AgentsIntent::SelectProfile(profile) => {
            profile_actions::select(state, &profile);
            Vec::new()
        },
        AgentsIntent::ToggleHiddenProfiles => {
            state.toggle_hidden_list();
            Vec::new()
        },
        AgentsIntent::LogIn(profile) => {
            profile_actions::log_in(state, &profile)
        },
        AgentsIntent::HideProfile(profile) => {
            profile_actions::set_hidden(state, &profile, true)
        },
        AgentsIntent::ShowProfile(profile) => {
            profile_actions::set_hidden(state, &profile, false)
        },
        AgentsIntent::RemoveProfile(profile) => {
            profile_actions::ask_removal(state, profile);
            Vec::new()
        },
        AgentsIntent::RemoveConfirmed => {
            profile_actions::confirm_removal(state)
        },
        AgentsIntent::RemoveCancelled => {
            let _ = state.take_removing();
            Vec::new()
        },
        AgentsIntent::OpenAddProfile => profile_dialog::open_add(state),
        AgentsIntent::EditProfile(profile) => {
            profile_dialog::open_edit(state, &profile)
        },
        AgentsIntent::Dialog(intent) => profile_dialog::reduce(state, intent),
        AgentsIntent::CandidatesLoaded { agent, folders } => {
            profile_dialog::take_candidates(state, agent, folders);
            Vec::new()
        },
        AgentsIntent::ProfilesSaved(result) => {
            profile_actions::saved(state, result)
        },
    }
}

/// Ask for the profiles, unless a read is on its way.
fn load_profiles(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    if !state.start_loading() {
        return Vec::new();
    }

    vec![AgentsOutcome::LoadProfiles]
}

/// Ask for the limits of each connected profile that has no read on its
/// way.
fn load_limits(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state
        .start_limit_reads()
        .into_iter()
        .map(AgentsOutcome::LoadLimits)
        .collect()
}

/// Ask for the tokens of each shown profile that has no count on its way.
fn load_spend(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state
        .start_spend_reads()
        .into_iter()
        .map(AgentsOutcome::LoadSpend)
        .collect()
}

/// Continue the session `id` of the project of `folder`, unless the folder
/// is missing.
fn continue_session(
    state: &AgentsState,
    folder: &Path,
    id: &SessionId,
) -> Vec<AgentsOutcome> {
    let Some(project) = state.project(folder) else {
        return Vec::new();
    };
    if state.is_missing(folder) {
        return Vec::new();
    }

    let entry = project
        .sessions()
        .iter()
        .find(|entry| entry.session().id() == id);
    let Some(entry) = entry else {
        return Vec::new();
    };

    open_tab(state, entry.profile(), Some(entry.session()), folder)
}

/// Start a new session in the selected project: right away with the only
/// profile that can start one, else ask which profile.
fn new_session(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    let profiles = state.startable_profiles();
    match profiles.as_slice() {
        [] => Vec::new(),
        [profile] => open_tab(state, profile, None, folder),
        _ => {
            state.set_choosing_profile(true);
            Vec::new()
        },
    }
}

/// Start a new session of `profile` in the selected project.
fn new_session_with(
    state: &AgentsState,
    profile: &Profile,
) -> Vec<AgentsOutcome> {
    let Some(folder) = openable_selected_folder(state) else {
        return Vec::new();
    };

    open_tab(state, profile, None, folder)
}

/// The folder of the selected project, while it is listed and exists.
fn openable_selected_folder(state: &AgentsState) -> Option<&Path> {
    let folder = state.selected_project()?.folder();
    if state.is_missing(folder) {
        return None;
    }

    Some(folder)
}

/// The terminal tab that starts `session` of `profile`, or a new session
/// when there is none, in `folder`. It is named after the session, or the
/// profile for a new one. A profile that can not start gives no tab.
fn open_tab(
    state: &AgentsState,
    profile: &Profile,
    session: Option<&AgentSession>,
    folder: &Path,
) -> Vec<AgentsOutcome> {
    if !state.can_start(profile) {
        return Vec::new();
    }
    let Some(agent) = state.agent(profile.agent()) else {
        log::debug!("no agent {} to start", profile.agent().as_str());
        return Vec::new();
    };

    let command = agent.launch(profile, session.map(AgentSession::id));
    let title = match session {
        Some(session) => session.title().to_string(),
        None => profile.name().to_string(),
    };

    vec![AgentsOutcome::OpenTab {
        title,
        folder: folder.to_path_buf(),
        command,
    }]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{AgentCommand, ProfileStatus, SessionId};

    use super::*;
    use crate::testing::{
        agents, at, claude, claude_work, codex, codex_work, connected, env,
        five_hour_limits, folders, limits_answer, limits_loaded, no_files,
        profiles_loaded, read, session, sessions_loaded, spend_loaded,
        spend_of, two_agents_with_sessions, with_profiles,
    };
    use crate::types::{LoadedProfile, LoadedProfiles, LoadedSessions};

    fn select(state: &mut AgentsState, folder: &str) {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
    }

    fn continue_session(
        state: &mut AgentsState,
        folder: &str,
        id: &str,
    ) -> Vec<AgentsOutcome> {
        reduce(
            state,
            AgentsIntent::Continue {
                folder: PathBuf::from(folder),
                session: SessionId::new(id),
            },
        )
    }

    fn open_tab(
        title: &str,
        folder: &str,
        command: AgentCommand,
    ) -> AgentsOutcome {
        AgentsOutcome::OpenTab {
            title: String::from(title),
            folder: PathBuf::from(folder),
            command,
        }
    }

    // Reading the profiles and the sessions.

    #[test]
    fn entering_the_mode_reads_the_profiles_once_at_a_time() {
        let mut state = AgentsState::new(agents(), env());

        let entered =
            reduce(&mut state, AgentsIntent::ModeChanged(Mode::Agents));
        let again = reduce(&mut state, AgentsIntent::Refresh);

        assert!(state.is_active());
        assert!(state.is_loading());
        assert!(!state.is_loaded());
        assert_eq!(entered, vec![AgentsOutcome::LoadProfiles]);
        assert!(again.is_empty(), "a read is on its way");
    }

    #[test]
    fn another_mode_leaves_agents_mode() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let left =
            reduce(&mut state, AgentsIntent::ModeChanged(Mode::Terminal));

        assert!(!state.is_active());
        assert!(left.is_empty());
    }

    #[test]
    fn each_shown_profile_has_its_sessions_read_and_a_connected_one_its_limits()
    {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let hidden = codex_work().with_hidden(true);

        let outcomes = reduce(
            &mut state,
            profiles_loaded(&[
                (claude(), connected()),
                (codex(), ProfileStatus::NotLoggedIn),
                (hidden, connected()),
                (claude_work(), ProfileStatus::ConfigDirNotFound),
            ]),
        );

        assert_eq!(
            outcomes,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadSessions(codex()),
                AgentsOutcome::LoadLimits(claude()),
                AgentsOutcome::LoadSpend(claude()),
                AgentsOutcome::LoadSpend(codex()),
            ],
            "only a connected profile has its limits read, every profile \
             with a config folder its tokens counted"
        );
        assert!(state.is_loaded());
        assert!(!state.is_loading());
        let shown: Vec<_> = state
            .profiles()
            .iter()
            .map(|shown| shown.profile().name())
            .collect();
        assert_eq!(shown, ["Claude Code", "Codex", "Claude Code · Work"]);
        assert!(state.profiles()[0].is_reading());
        assert!(!state.profiles()[2].is_reading());
    }

    #[test]
    fn a_problem_with_the_saved_profiles_is_told() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: Vec::new(),
                saved: None,
                problem: Some(String::from("Could not read")),
            }),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::Failed(String::from("Could not read"))]
        );
    }

    #[test]
    fn loaded_sessions_become_the_projects_list() {
        let state = two_agents_with_sessions();

        assert_eq!(
            folders(&state),
            [Path::new("/work/site"), Path::new("/work/otty")]
        );
        let otty = &state.projects()[1];
        let ids: Vec<_> = otty
            .sessions()
            .iter()
            .map(|entry| entry.session().id().as_str())
            .collect();
        assert_eq!(ids, ["x1", "c1"]);
        assert_eq!(otty.sessions()[0].profile(), &codex());
        assert!(!state.profiles()[0].is_reading());
    }

    #[test]
    fn a_failure_of_one_profile_keeps_the_other_profiles_sessions() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );

        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: codex(),
                sessions: Err(vec![no_files()]),
                missing_folders: Vec::new(),
            }),
        );

        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert_eq!(state.profiles()[0].failures(), None);
        assert_eq!(state.profiles()[1].failures(), Some(&[no_files()][..]));
        assert!(!state.profiles()[1].is_reading());
    }

    #[test]
    fn a_failed_read_keeps_the_sessions_read_before() {
        let mut state = with_profiles(&[(claude(), connected())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, profiles_loaded(&[(claude(), connected())]));
        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Err(vec![no_files()]),
                missing_folders: Vec::new(),
            }),
        );

        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert!(state.profiles()[0].failures().is_some());
    }

    #[test]
    fn an_agent_that_can_not_list_sessions_has_none_and_no_failure() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: Err(Vec::new()),
                missing_folders: Vec::new(),
            }),
        );

        assert!(state.projects().is_empty());
        assert_eq!(state.profiles()[0].failures(), None);
        assert!(!state.profiles()[0].is_reading());
    }

    #[test]
    fn sessions_of_a_profile_no_longer_shown_are_dropped() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(
            &mut state,
            sessions_loaded(&codex(), vec![session("x1", "/work/otty", 1)]),
        );

        assert!(state.projects().is_empty());
    }

    // Reading the limits.

    #[test]
    fn the_tick_reads_the_limits_again_unless_a_read_is_on_its_way() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (codex(), connected()),
            (codex_work(), ProfileStatus::NotLoggedIn),
        ]);

        let while_reading = reduce(&mut state, AgentsIntent::LimitsTick);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
                at(10),
            ),
        );
        let after_one_answer = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(while_reading.is_empty(), "both reads are on their way");
        assert_eq!(after_one_answer, vec![AgentsOutcome::LoadLimits(claude())]);
    }

    #[test]
    fn the_tick_reads_nothing_outside_agents_mode() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            limits_loaded(
                &claude(),
                limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
                at(10),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::Leave);

        assert!(reduce(&mut state, AgentsIntent::LimitsTick).is_empty());
    }

    #[test]
    fn the_limits_are_kept_by_their_profile_and_survive_a_refresh() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);
        let answer =
            limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]);

        let _ =
            reduce(&mut state, limits_loaded(&codex(), answer.clone(), at(10)));
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = reduce(
            &mut state,
            profiles_loaded(&[(claude(), connected()), (codex(), connected())]),
        );

        let expected = answer.expect("the source answers");
        assert_eq!(state.profiles()[0].limits().last_good(), None);
        assert_eq!(
            state.profiles()[1].limits().last_good(),
            Some((&expected, at(10)))
        );
        assert!(
            refreshed.contains(&AgentsOutcome::LoadLimits(codex())),
            "Refresh reads the limits again"
        );
        assert!(
            !refreshed.contains(&AgentsOutcome::LoadLimits(claude())),
            "a read is still on its way"
        );
    }

    // Counting the tokens.

    #[test]
    fn the_tokens_are_kept_by_their_profile_and_counted_again_on_refresh() {
        let mut state =
            with_profiles(&[(claude(), connected()), (codex(), connected())]);

        let _ = reduce(
            &mut state,
            spend_loaded(&codex(), Some(Ok(spend_of(10, 300))), at(10)),
        );
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = reduce(
            &mut state,
            profiles_loaded(&[(claude(), connected()), (codex(), connected())]),
        );

        assert_eq!(state.profiles()[0].spend().last_good(), None);
        assert_eq!(
            state.profiles()[1].spend().last_good(),
            Some((&spend_of(10, 300), at(10)))
        );
        assert!(
            refreshed.contains(&AgentsOutcome::LoadSpend(codex())),
            "Refresh counts the tokens again"
        );
        assert!(
            !refreshed.contains(&AgentsOutcome::LoadSpend(claude())),
            "a count is still on its way"
        );
    }

    #[test]
    fn the_tick_of_the_limits_does_not_count_the_tokens() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            spend_loaded(&claude(), Some(Ok(spend_of(1, 2))), at(10)),
        );

        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(!ticked.contains(&AgentsOutcome::LoadSpend(claude())));
    }

    #[test]
    fn tokens_of_a_profile_no_longer_shown_are_dropped() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let outcomes = reduce(
            &mut state,
            spend_loaded(&codex(), Some(Ok(spend_of(1, 2))), at(10)),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.profiles()[0].spend().last_good(), None);
    }

    #[test]
    fn limits_of_a_profile_no_longer_shown_are_dropped() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let outcomes = reduce(
            &mut state,
            limits_loaded(
                &codex(),
                limits_answer(vec![("server", Ok(five_hour_limits(42.0)))]),
                at(10),
            ),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.profiles()[0].limits().last_good(), None);
        assert!(state.profiles()[0].limits().is_reading());
    }

    // Continue and New session.

    #[test]
    fn continue_starts_the_session_of_a_default_profile_with_no_variable() {
        let mut state = two_agents_with_sessions();

        let claude_tab = continue_session(&mut state, "/work/otty", "c1");
        let codex_tab = continue_session(&mut state, "/work/otty", "x1");

        assert_eq!(
            claude_tab,
            vec![open_tab(
                "Title of c1",
                "/work/otty",
                AgentCommand::new("claude")
                    .with_arg("--resume")
                    .with_arg("c1"),
            )]
        );
        assert_eq!(
            codex_tab,
            vec![open_tab(
                "Title of x1",
                "/work/otty",
                AgentCommand::new("codex").with_arg("resume").with_arg("x1"),
            )]
        );
    }

    #[test]
    fn continue_starts_the_session_of_an_added_profile_with_its_folder() {
        let mut state = with_profiles(&[
            (claude_work(), connected()),
            (codex_work(), connected()),
        ]);
        read(
            &mut state,
            &claude_work(),
            vec![session("c1", "/work/otty", 1)],
        );
        read(
            &mut state,
            &codex_work(),
            vec![session("x1", "/work/otty", 2)],
        );

        let claude_tab = continue_session(&mut state, "/work/otty", "c1");
        let codex_tab = continue_session(&mut state, "/work/otty", "x1");

        assert_eq!(
            claude_tab,
            vec![open_tab(
                "Title of c1",
                "/work/otty",
                AgentCommand::new("claude")
                    .with_arg("--resume")
                    .with_arg("c1")
                    .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work"),
            )]
        );
        assert_eq!(
            codex_tab,
            vec![open_tab(
                "Title of x1",
                "/work/otty",
                AgentCommand::new("codex")
                    .with_arg("resume")
                    .with_arg("x1")
                    .with_env("CODEX_HOME", "/home/user/.codex-work"),
            )]
        );
    }

    #[test]
    fn a_new_session_with_one_profile_starts_right_away() {
        let mut state = with_profiles(&[(claude_work(), connected())]);
        read(
            &mut state,
            &claude_work(),
            vec![session("c1", "/work/otty", 1)],
        );
        select(&mut state, "/work/otty");

        let outcomes = reduce(&mut state, AgentsIntent::NewSession);

        assert_eq!(
            outcomes,
            vec![open_tab(
                "Claude Code · Work",
                "/work/otty",
                AgentCommand::new("claude")
                    .with_env("CLAUDE_CONFIG_DIR", "/home/user/.claude-work"),
            )]
        );
        assert!(!state.is_choosing_profile());
    }

    #[test]
    fn a_new_session_with_two_profiles_asks_which_one() {
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/site");

        let asked = reduce(&mut state, AgentsIntent::NewSession);
        assert!(asked.is_empty());
        assert!(state.is_choosing_profile());

        let started = reduce(&mut state, AgentsIntent::NewSessionWith(codex()));

        assert_eq!(
            started,
            vec![open_tab("Codex", "/work/site", AgentCommand::new("codex"))]
        );
        assert!(!state.is_choosing_profile());
    }

    #[test]
    fn the_choice_of_a_profile_can_be_cancelled_and_ends_with_the_mode() {
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/site");

        let _ = reduce(&mut state, AgentsIntent::NewSession);
        let _ = reduce(&mut state, AgentsIntent::CancelNewSession);
        assert!(!state.is_choosing_profile());

        let _ = reduce(&mut state, AgentsIntent::NewSession);
        let _ = reduce(&mut state, AgentsIntent::Leave);
        assert!(!state.is_choosing_profile());

        let _ = reduce(&mut state, AgentsIntent::NewSession);
        select(&mut state, "/work/otty");
        assert!(!state.is_choosing_profile());
    }

    #[test]
    fn a_new_session_needs_a_selected_project() {
        let mut state = two_agents_with_sessions();

        let outcomes = reduce(&mut state, AgentsIntent::NewSession);
        let chosen = reduce(&mut state, AgentsIntent::NewSessionWith(codex()));

        assert!(outcomes.is_empty());
        assert!(chosen.is_empty());
        assert!(!state.is_choosing_profile());
    }

    #[test]
    fn a_session_of_a_missing_project_can_not_be_opened() {
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

        let continued = continue_session(&mut state, "/work/gone", "c1");
        let started = reduce(&mut state, AgentsIntent::NewSession);
        let chosen = reduce(&mut state, AgentsIntent::NewSessionWith(claude()));

        assert!(state.is_missing(Path::new("/work/gone")));
        assert!(continued.is_empty());
        assert!(started.is_empty());
        assert!(chosen.is_empty());
    }

    #[test]
    fn a_profile_whose_program_is_not_found_starts_nothing() {
        let mut state = with_profiles(&[
            (claude(), ProfileStatus::ProgramNotFound),
            (codex(), connected()),
        ]);
        read(&mut state, &claude(), vec![session("c1", "/work/otty", 1)]);
        select(&mut state, "/work/otty");

        let continued = continue_session(&mut state, "/work/otty", "c1");
        let chosen = reduce(&mut state, AgentsIntent::NewSessionWith(claude()));
        let started = reduce(&mut state, AgentsIntent::NewSession);

        assert!(continued.is_empty());
        assert!(chosen.is_empty());
        assert_eq!(
            started,
            vec![open_tab("Codex", "/work/otty", AgentCommand::new("codex"))],
            "Codex is the only profile that can start"
        );
    }

    #[test]
    fn an_unknown_session_or_project_starts_nothing() {
        let mut state = two_agents_with_sessions();

        assert!(continue_session(&mut state, "/work/otty", "nope").is_empty());
        assert!(continue_session(&mut state, "/work/none", "c1").is_empty());
    }

    #[test]
    fn a_profile_of_an_unknown_agent_starts_nothing() {
        let stranger = Profile::added(
            otty_agents::AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let mut state = with_profiles(&[(stranger.clone(), connected())]);
        read(&mut state, &stranger, vec![session("s1", "/work/otty", 1)]);

        assert!(continue_session(&mut state, "/work/otty", "s1").is_empty());
    }

    #[test]
    fn profiles_read_again_replace_the_old_ones() {
        let mut state = two_agents_with_sessions();

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let outcomes = reduce(
            &mut state,
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: vec![LoadedProfile {
                    profile: codex(),
                    status: connected(),
                }],
                saved: Some(otty_agents::SavedProfiles::default()),
                problem: None,
            }),
        );

        assert_eq!(outcomes, vec![AgentsOutcome::LoadSessions(codex())]);
        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert!(state.profiles()[0].is_reading());
    }
}
