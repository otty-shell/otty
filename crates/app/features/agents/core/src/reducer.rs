use otty_app_domain::mode::Mode;

use super::event::{AgentsIntent, AgentsOutcome};
use super::menus::AgentsMenu;
use super::state::AgentsState;
use super::{profile_actions, profile_dialog, session_actions};

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
            outcomes.extend(load_bills(state));
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
        AgentsIntent::BillLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_bill(loaded) {
                log::debug!(
                    "agent bill of a profile no longer shown dropped: {name}"
                );
            }
            Vec::new()
        },
        AgentsIntent::LimitsTick if state.is_active() => {
            let mut outcomes = load_limits(state);
            outcomes.extend(load_bills(state));
            outcomes
        },
        AgentsIntent::LimitsTick => Vec::new(),
        AgentsIntent::SelectProject(folder) => {
            state.select_project(folder);
            Vec::new()
        },
        AgentsIntent::SessionSearch(action) => {
            session_actions::search(state, action);
            Vec::new()
        },
        AgentsIntent::SortSessions(column) => {
            state.table_mut().press_sort(column);
            Vec::new()
        },
        AgentsIntent::SelectSession(index) => {
            let _ = session_actions::select(state, index);
            Vec::new()
        },
        AgentsIntent::OpenSession(index) => session_actions::open(state, index),
        AgentsIntent::ContinueSelected => {
            session_actions::continue_selected(state)
        },
        AgentsIntent::OpenSessionMenu(index) => {
            session_actions::open_menu(state, index)
        },
        AgentsIntent::SessionMenu(action) => {
            session_actions::act(state, action)
        },
        AgentsIntent::OpenProjectFolder => {
            session_actions::open_project_folder(state)
        },
        AgentsIntent::NewSession => session_actions::new_session(state),
        AgentsIntent::NewSessionWith(profile) => {
            let _ = state.close_menu();
            session_actions::new_session_with(state, &profile)
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
        AgentsIntent::OpenProfileMenu(profile) => {
            state.open_menu(AgentsMenu::Profile(profile));
            vec![AgentsOutcome::MenuOpened]
        },
        AgentsIntent::HideProfile(profile) => {
            let _ = state.close_menu();
            profile_actions::set_hidden(state, &profile, true)
        },
        AgentsIntent::ShowProfile(profile) => {
            profile_actions::set_hidden(state, &profile, false)
        },
        AgentsIntent::RemoveProfile(profile) => {
            let _ = state.close_menu();
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
        AgentsIntent::ToggleAgentsSection => {
            state.toggle_agents_section();
            Vec::new()
        },
        AgentsIntent::ToggleProjectsSection => {
            state.toggle_projects_section();
            Vec::new()
        },
        AgentsIntent::ToggleProjectSearch => {
            if !state.search_mut().toggle() {
                return Vec::new();
            }
            vec![AgentsOutcome::FocusProjectSearch]
        },
        AgentsIntent::ProjectSearchChanged(query) => {
            state.search_mut().set_query(query);
            Vec::new()
        },
        AgentsIntent::ProjectSearchCleared => {
            state.search_mut().clear_or_close();
            Vec::new()
        },
        AgentsIntent::ProjectSearchSubmitted(first) => {
            if let Some(folder) = first {
                state.select_project(folder);
            }
            Vec::new()
        },
        AgentsIntent::OpenFilterMenu => {
            state.open_menu(AgentsMenu::ProjectFilter);
            vec![AgentsOutcome::MenuOpened]
        },
        AgentsIntent::ProjectFilter(change) => {
            state.search_mut().change_filter(change);
            let _ = state.close_menu();
            Vec::new()
        },
        AgentsIntent::OpenNewSessionMenu => {
            session_actions::open_new_session_menu(state)
        },
        AgentsIntent::CloseMenu => {
            let _ = state.close_menu();
            Vec::new()
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

/// Ask for the bill of each API key profile with an admin key that has no
/// read on its way.
fn load_bills(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    state
        .start_bill_reads()
        .into_iter()
        .map(AgentsOutcome::LoadBill)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use iced::{Point, Size};
    use otty_agents::{AgentCommand, Profile, ProfileStatus};

    use super::*;
    use crate::menus::MenuPlace;
    use crate::project_filter::ProjectFilterChange;
    use crate::testing::{
        agents, api_key, at, bill_loaded, bill_of, claude,
        claude_with_admin_key, claude_work, codex, codex_work, connected, env,
        five_hour_limits, folders, limits_answer, limits_loaded, no_files, now,
        profiles_loaded, read, session, sessions_loaded, spend_loaded,
        spend_of, two_agents_with_sessions, with_profiles,
    };
    use crate::types::{LoadedProfile, LoadedProfiles, LoadedSessions};

    fn select(state: &mut AgentsState, folder: &str) {
        let _ =
            reduce(state, AgentsIntent::SelectProject(PathBuf::from(folder)));
    }

    /// Select the project of `folder` and double-press the row of its
    /// session `id`; a session the table does not show is a row past the
    /// end.
    fn continue_session(
        state: &mut AgentsState,
        folder: &str,
        id: &str,
    ) -> Vec<AgentsOutcome> {
        select(state, folder);
        let index = state
            .shown_sessions()
            .iter()
            .position(|session| session.id().as_str() == id);

        reduce(
            state,
            AgentsIntent::OpenSession(index.unwrap_or(usize::MAX)),
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

    // Reading the bill.

    #[test]
    fn an_api_key_profile_with_an_admin_key_has_its_bill_read() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let codex_with_key = codex()
            .with_admin_key(claude_with_admin_key().admin_key().cloned());

        let outcomes = reduce(
            &mut state,
            profiles_loaded(&[
                (claude_with_admin_key(), api_key()),
                (claude_work(), api_key()),
                (codex_with_key.clone(), api_key()),
            ]),
        );

        let bills: Vec<_> = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadBill(_)))
            .collect();
        assert_eq!(
            bills,
            [&AgentsOutcome::LoadBill(claude_with_admin_key())],
            "a profile with no admin key, or of an agent with no bill, \
             has no bill read"
        );
    }

    #[test]
    fn a_subscription_with_an_admin_key_has_no_bill_read() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            profiles_loaded(&[(claude_with_admin_key(), connected())]),
        );

        assert!(
            !outcomes
                .contains(&AgentsOutcome::LoadBill(claude_with_admin_key()))
        );
    }

    #[test]
    fn the_bill_is_kept_by_its_profile_and_read_again_on_the_tick() {
        let mut state = with_profiles(&[(claude_with_admin_key(), api_key())]);
        let while_reading = reduce(&mut state, AgentsIntent::LimitsTick);

        let applied = reduce(
            &mut state,
            bill_loaded(&claude_with_admin_key(), Ok(bill_of(12, 3))),
        );
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(while_reading.is_empty(), "a read is on its way");
        assert!(applied.is_empty());
        assert_eq!(state.profiles()[0].bill().answer(), Some(&bill_of(12, 3)));
        assert_eq!(
            ticked,
            vec![AgentsOutcome::LoadBill(claude_with_admin_key())]
        );
    }

    #[test]
    fn a_bill_of_a_profile_no_longer_shown_is_dropped() {
        let mut state = with_profiles(&[(claude_with_admin_key(), api_key())]);

        let outcomes =
            reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(1, 2))));

        assert!(outcomes.is_empty());
        assert_eq!(state.profiles()[0].bill().answer(), None);
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
        assert_eq!(open_menu(&state), None);
    }

    /// Coordinator, from issue 14: with more than one profile the plus
    /// opens the same list of the profiles as the chevron, at the pointer.
    #[test]
    fn a_new_session_with_two_profiles_opens_the_profiles_at_the_pointer() {
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/site");
        state.set_pointer(place());

        let asked = reduce(&mut state, AgentsIntent::NewSession);
        assert_eq!(asked, vec![AgentsOutcome::MenuOpened]);
        assert_eq!(open_menu(&state), Some(AgentsMenu::NewSession));
        assert_eq!(state.menu().map(|open| open.place()), Some(place()));

        let started = reduce(&mut state, AgentsIntent::NewSessionWith(codex()));

        assert_eq!(
            started,
            vec![open_tab("Codex", "/work/site", AgentCommand::new("codex"))]
        );
        assert_eq!(open_menu(&state), None);
    }

    #[test]
    fn a_new_session_needs_a_selected_project() {
        let mut state = two_agents_with_sessions();

        let outcomes = reduce(&mut state, AgentsIntent::NewSession);
        let chosen = reduce(&mut state, AgentsIntent::NewSessionWith(codex()));

        assert!(outcomes.is_empty());
        assert!(chosen.is_empty());
        assert_eq!(open_menu(&state), None);
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
        assert!(state.table().selected().is_none());
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

    // The sections, the search field and the menus of the left panel.

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(120.0, 300.0), Size::new(1200.0, 800.0))
    }

    fn open_menu(state: &AgentsState) -> Option<AgentsMenu> {
        state.menu().map(|open| open.menu().clone())
    }

    #[test]
    fn an_api_key_profile_has_its_tokens_counted_and_no_limits_read() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let loaded =
            reduce(&mut state, profiles_loaded(&[(claude(), api_key())]));
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert_eq!(
            loaded,
            vec![
                AgentsOutcome::LoadSessions(claude()),
                AgentsOutcome::LoadSpend(claude()),
            ]
        );
        assert!(ticked.is_empty());
    }

    #[test]
    fn the_more_button_opens_the_profile_menu_and_its_items_close_it() {
        let mut state = with_profiles(&[(claude_work(), connected())]);
        state.set_pointer(place());

        let opened =
            reduce(&mut state, AgentsIntent::OpenProfileMenu(claude_work()));
        let menu = open_menu(&state);
        let hidden =
            reduce(&mut state, AgentsIntent::HideProfile(claude_work()));
        let after_hide = open_menu(&state);
        let _ =
            reduce(&mut state, AgentsIntent::OpenProfileMenu(claude_work()));
        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude_work()));

        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert_eq!(menu, Some(AgentsMenu::Profile(claude_work())));
        assert!(matches!(
            hidden.as_slice(),
            [AgentsOutcome::SaveProfiles { .. }]
        ));
        assert_eq!(after_hide, None);
        assert_eq!(open_menu(&state), None);
        assert_eq!(state.removing(), Some(&claude_work()));
    }

    #[test]
    fn each_section_opens_and_closes_on_its_own() {
        let mut state = two_agents_with_sessions();
        assert!(state.is_agents_open());
        assert!(state.is_projects_open());

        let _ = reduce(&mut state, AgentsIntent::ToggleAgentsSection);
        assert!(!state.is_agents_open());
        assert!(state.is_projects_open());

        let _ = reduce(&mut state, AgentsIntent::ToggleProjectsSection);
        let _ = reduce(&mut state, AgentsIntent::ToggleAgentsSection);
        assert!(state.is_agents_open());
        assert!(!state.is_projects_open());
    }

    #[test]
    fn the_search_button_opens_the_field_with_the_focus_and_closes_it() {
        let mut state = two_agents_with_sessions();

        let opened = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        assert_eq!(opened, vec![AgentsOutcome::FocusProjectSearch]);
        assert!(state.search().is_open());

        let closed = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        assert!(closed.is_empty());
        assert!(!state.search().is_open());
    }

    #[test]
    fn typing_filters_the_projects_at_once() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectSearchChanged(String::from("SIT")),
        );

        assert_eq!(state.search().query(), "SIT");
        let shown: Vec<&Path> = state
            .search()
            .shown(state.projects(), |_| false, now())
            .iter()
            .map(|project| project.folder())
            .collect();
        assert_eq!(shown, [Path::new("/work/site")]);
    }

    #[test]
    fn enter_in_the_search_selects_the_first_project_shown_or_nothing() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);

        let none =
            reduce(&mut state, AgentsIntent::ProjectSearchSubmitted(None));
        let nothing_selected = state.selected_folder().is_none();
        let first = reduce(
            &mut state,
            AgentsIntent::ProjectSearchSubmitted(Some(PathBuf::from(
                "/work/otty",
            ))),
        );

        assert!(none.is_empty());
        assert!(nothing_selected);
        assert!(first.is_empty());
        assert_eq!(state.selected_folder(), Some(Path::new("/work/otty")));
        assert!(state.search().is_open(), "the search stays");
    }

    #[test]
    fn esc_clears_the_text_and_esc_on_an_empty_field_closes_it() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectSearchChanged(String::from("otty")),
        );

        let _ = reduce(&mut state, AgentsIntent::ProjectSearchCleared);
        assert_eq!(state.search().query(), "");
        assert!(state.search().is_open(), "the first Esc only clears");

        let _ = reduce(&mut state, AgentsIntent::ProjectSearchCleared);
        assert!(!state.search().is_open(), "the second Esc closes");
    }

    #[test]
    fn a_closed_field_keeps_its_text_and_choices_and_hides_no_project() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectSearchChanged(String::from("site")),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectFilter(ProjectFilterChange::SortByName(true)),
        );

        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        let shown = state.search().shown(state.projects(), |_| false, now());
        assert_eq!(shown.len(), 2);

        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        assert_eq!(state.search().query(), "site");
        assert!(state.search().filter().is_sorted_by_name());
    }

    #[test]
    fn the_funnel_menu_opens_at_the_pointer_and_a_choice_closes_it() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        state.set_pointer(place());

        let opened = reduce(&mut state, AgentsIntent::OpenFilterMenu);
        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert_eq!(open_menu(&state), Some(AgentsMenu::ProjectFilter));
        assert_eq!(state.menu().map(|open| open.place()), Some(place()));

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectFilter(ProjectFilterChange::Agent(Some(
                crate::testing::CODEX,
            ))),
        );

        assert_eq!(open_menu(&state), None);
        assert_eq!(
            state.search().filter().agent(),
            Some(crate::testing::CODEX)
        );
    }

    #[test]
    fn the_cross_of_a_chip_turns_its_choice_off() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        for change in [
            ProjectFilterChange::RecentOnly(true),
            ProjectFilterChange::MissingOnly(true),
        ] {
            let _ = reduce(&mut state, AgentsIntent::ProjectFilter(change));
        }

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectFilter(ProjectFilterChange::RecentOnly(false)),
        );
        assert!(!state.search().filter().is_recent_only());
        assert!(state.search().filter().is_missing_only());

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectFilter(ProjectFilterChange::ClearAll),
        );
        assert!(!state.search().filter().is_any_on());
    }

    #[test]
    fn a_menu_closes_on_close_menu_and_when_the_mode_is_left() {
        let mut state = two_agents_with_sessions();

        let _ = reduce(&mut state, AgentsIntent::OpenFilterMenu);
        let _ = reduce(&mut state, AgentsIntent::CloseMenu);
        assert_eq!(open_menu(&state), None);

        let _ = reduce(&mut state, AgentsIntent::OpenFilterMenu);
        let _ = reduce(&mut state, AgentsIntent::Leave);
        assert_eq!(open_menu(&state), None);
    }

    #[test]
    fn the_new_session_menu_needs_a_project_that_can_start_a_session() {
        let mut state = two_agents_with_sessions();

        let without_project =
            reduce(&mut state, AgentsIntent::OpenNewSessionMenu);
        assert!(without_project.is_empty());
        assert_eq!(open_menu(&state), None);

        select(&mut state, "/work/site");
        let opened = reduce(&mut state, AgentsIntent::OpenNewSessionMenu);
        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert_eq!(open_menu(&state), Some(AgentsMenu::NewSession));

        let started = reduce(&mut state, AgentsIntent::NewSessionWith(codex()));
        assert_eq!(
            started,
            vec![open_tab("Codex", "/work/site", AgentCommand::new("codex"))]
        );
        assert_eq!(open_menu(&state), None);
    }

    #[test]
    fn selecting_a_row_of_the_filtered_list_keeps_the_search() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::ToggleProjectSearch);
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectSearchChanged(String::from("otty")),
        );

        select(&mut state, "/work/otty");
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        select(&mut state, "/work/otty");

        assert_eq!(
            state.selected_project().map(|project| project.folder()),
            Some(Path::new("/work/otty"))
        );
        assert!(state.search().is_open());
        assert_eq!(state.search().query(), "otty");
    }
}
