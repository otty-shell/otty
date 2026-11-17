use std::time::SystemTime;

use otty_app_domain::mode::Mode;

use super::event::{AgentsIntent, AgentsOutcome};
use super::menus::AgentsMenu;
use super::state::AgentsState;
use super::{
    new_session_dialog, profile_actions, profile_dialog, reads, real_failures,
    session_actions, stats_dialog,
};

/// Reduce an Agents intent into state changes and the outcomes the binary
/// reads. The reducer reads no file and starts no program: it asks for the
/// reads and for the terminal tab with outcomes, which reads and when is
/// decided in `reads.rs`, and each answer is checked for a real failure to
/// tell (`real_failures.rs`).
pub(super) fn reduce(
    state: &mut AgentsState,
    intent: AgentsIntent,
) -> Vec<AgentsOutcome> {
    match intent {
        AgentsIntent::ModeChanged(Mode::Agents) => {
            reduce(state, AgentsIntent::Enter)
        },
        AgentsIntent::ModeChanged(_) => reduce(state, AgentsIntent::Leave),
        AgentsIntent::LoadInBackground => reads::start_in_background(state),
        AgentsIntent::Enter => {
            state.set_active(true);
            reads::on_open(state, SystemTime::now())
        },
        AgentsIntent::Leave => {
            state.set_active(false);
            Vec::new()
        },
        AgentsIntent::Refresh => reads::load_profiles(state),
        AgentsIntent::ProfilesLoaded(loaded) => {
            let check = real_failures::profiles(loaded.problem.as_deref());
            let mut outcomes = state.told_mut().tell(vec![check]);

            state.set_saved(loaded.saved);
            state.remember_utc_offset(loaded.utc_offset_seconds);
            state.remember_profiles_read_at(loaded.read_at);
            let to_read = state.apply_profiles(loaded.profiles);
            // A read must follow when `agents.json` changed during this
            // one, which may have read it before: a profile just saved and
            // selected may be missing from it, so the selection waits.
            let is_reload_wanted = state.take_reload_wanted();
            if !is_reload_wanted {
                state.forget_selection_that_left();
            }
            state.select_first_profile_if_none();

            outcomes
                .extend(to_read.into_iter().map(AgentsOutcome::LoadSessions));
            outcomes.extend(reads::after_profiles(state));
            if is_reload_wanted {
                outcomes.extend(reads::load_profiles(state));
            }
            outcomes
        },
        AgentsIntent::SessionsLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            let check = real_failures::sessions(&loaded);
            if !state.apply_sessions(loaded) {
                log::debug!(
                    "agent sessions of a profile no longer shown dropped: {name}"
                );
                return Vec::new();
            }

            state.forget_selection_that_left();
            state.select_first_profile_if_none();

            let mut outcomes = state.told_mut().tell(vec![check]);
            outcomes.extend(reads::go_on_at_start(state));
            outcomes
        },
        AgentsIntent::LimitsLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            let checks = real_failures::limits(&loaded);
            if !state.apply_limits(loaded) {
                log::debug!(
                    "agent limits of a profile no longer shown dropped: {name}"
                );
                return Vec::new();
            }

            let mut outcomes = state.told_mut().tell(checks.into());
            outcomes.extend(reads::after_limits(state));
            outcomes
        },
        AgentsIntent::SpendLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_spend(loaded) {
                log::debug!(
                    "agent tokens of a profile no longer shown dropped: {name}"
                );
                return Vec::new();
            }
            reads::go_on_at_start(state)
        },
        AgentsIntent::BillLoaded(loaded) => {
            let name = loaded.profile.name().to_string();
            if !state.apply_bill(loaded) {
                log::debug!(
                    "agent bill of a profile no longer shown dropped: {name}"
                );
                return Vec::new();
            }
            reads::go_on_at_start(state)
        },
        AgentsIntent::PricesLoaded(loaded) => {
            let check = real_failures::prices(&loaded);
            state.apply_prices(loaded);

            let mut outcomes = state.told_mut().tell(vec![check]);
            outcomes.extend(reads::go_on_at_start(state));
            outcomes
        },
        AgentsIntent::LimitsTick => reads::on_tick(state),
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
        AgentsIntent::ShowProjectStats => stats_dialog::show_project(state),
        AgentsIntent::SessionStatsLoaded(loaded) => {
            stats_dialog::apply_session(state, loaded);
            Vec::new()
        },
        AgentsIntent::ProjectStatsLoaded(loaded) => {
            stats_dialog::apply_project(state, loaded);
            Vec::new()
        },
        AgentsIntent::CloseStats => {
            let _ = state.close_stats();
            Vec::new()
        },
        AgentsIntent::NewSession => session_actions::new_session(state),
        AgentsIntent::NewSessionWith(profile) => {
            let _ = state.close_menu();
            session_actions::new_session_with(state, &profile)
        },
        AgentsIntent::OpenNewSessionDialog(folder) => {
            new_session_dialog::open(state, folder)
        },
        AgentsIntent::NewSessionDialog(intent) => {
            new_session_dialog::reduce(state, intent)
        },
        AgentsIntent::FolderLookedUp(loaded) => {
            new_session_dialog::apply_lookup(state, loaded);
            Vec::new()
        },
        AgentsIntent::SelectProfile(profile) => {
            profile_actions::select(state, &profile);
            Vec::new()
        },
        AgentsIntent::LogIn(profile) => {
            let _ = state.close_menu();
            profile_actions::log_in(state, &profile)
        },
        AgentsIntent::OpenProfileMenu(profile) => {
            state.open_menu(AgentsMenu::Profile(profile));
            vec![AgentsOutcome::MenuOpened]
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
            let _ = state.close_menu();
            profile_dialog::open_edit(state, &profile)
        },
        AgentsIntent::Dialog(intent) => profile_dialog::reduce(state, intent),
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
        AgentsIntent::ShowTop(tab) => {
            state.show_top(tab);
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

        let outcomes = reduce(
            &mut state,
            profiles_loaded(&[
                (claude(), connected()),
                (codex(), ProfileStatus::NotLoggedIn),
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
                utc_offset_seconds: None,
                read_at: at(0),
            }),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::Failed(String::from("Could not read"))]
        );
    }

    /// The first read happens at the start of OTTY with the mode away; the
    /// mode then opens on the first profile of the list.
    #[test]
    fn the_first_read_selects_the_first_profile() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let _ = reduce(
            &mut state,
            profiles_loaded(&[(codex(), connected()), (claude(), connected())]),
        );

        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&codex())
        );
    }

    #[test]
    fn with_no_profile_nothing_is_selected_until_one_is_found() {
        let mut state = with_profiles(&[]);
        let nothing = state.selected_profile().is_none();

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, profiles_loaded(&[(claude(), connected())]));

        assert!(nothing);
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&claude())
        );
    }

    #[test]
    fn a_refresh_keeps_the_profile_the_user_selected() {
        let shown = [(claude(), connected()), (codex(), connected())];
        let mut state = with_profiles(&shown);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(codex()));

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, profiles_loaded(&shown));

        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&codex())
        );
    }

    #[test]
    fn a_refresh_keeps_the_project_the_user_selected() {
        let shown = [(claude(), connected()), (codex(), connected())];
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/otty");

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, profiles_loaded(&shown));

        assert!(state.selected_profile().is_none());
        assert_eq!(state.selected_folder(), Some(Path::new("/work/otty")));
    }

    #[test]
    fn a_selected_profile_that_left_the_list_gives_way_to_the_first_one() {
        let mut state = with_profiles(&[
            (claude(), connected()),
            (claude_work(), connected()),
        ]);
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude_work()));

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(&mut state, profiles_loaded(&[(claude(), connected())]));

        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&claude())
        );
    }

    /// `/work/site` has only a session of Claude Code, which a later read
    /// no longer finds.
    #[test]
    fn a_selected_project_that_left_the_list_gives_way_to_the_first_profile() {
        let mut state = two_agents_with_sessions();
        select(&mut state, "/work/site");

        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );

        assert_eq!(state.selected_folder(), None);
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&claude())
        );
    }

    /// A save selected its new profile while an older read was on its way:
    /// that read may not have it yet, so the selection waits for the read
    /// that follows.
    #[test]
    fn a_profile_selected_before_it_is_read_stays_until_the_read_after_it() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        state.select_named(claude_work().agent(), claude_work().name());
        let _ = state.start_reload();

        let older =
            reduce(&mut state, profiles_loaded(&[(claude(), connected())]));
        let kept = state.selected_profile().is_none();
        let _ = reduce(
            &mut state,
            profiles_loaded(&[
                (claude(), connected()),
                (claude_work(), connected()),
            ]),
        );

        assert!(older.contains(&AgentsOutcome::LoadProfiles));
        assert!(kept, "the new profile is not shown yet");
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&claude_work())
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
                limits_answer(vec![(
                    otty_agents::SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
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
                limits_answer(vec![(
                    otty_agents::SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
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
        let answer = limits_answer(vec![(
            otty_agents::SourceId::ChatGptUsageApi,
            Ok(five_hour_limits(42.0)),
        )]);

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
    fn the_offset_from_utc_read_with_the_profiles_is_kept_until_known_again() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let read = |offset: Option<i32>| {
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: Vec::new(),
                saved: None,
                problem: None,
                utc_offset_seconds: offset,
                read_at: at(0),
            })
        };
        let before = state.utc_offset_seconds();

        let _ = reduce(&mut state, read(Some(3 * 3600)));
        let _ = reduce(&mut state, read(None));

        assert_eq!(before, None, "not known before the first read");
        assert_eq!(state.utc_offset_seconds(), Some(10_800));
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
                limits_answer(vec![(
                    otty_agents::SourceId::ChatGptUsageApi,
                    Ok(five_hour_limits(42.0)),
                )]),
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
                utc_offset_seconds: None,
                read_at: at(0),
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
                AgentsOutcome::LoadPrices { fetch_again: false },
            ]
        );
        assert!(ticked.is_empty());
    }

    #[test]
    fn the_more_button_opens_the_profile_menu_and_its_items_close_it() {
        let mut state =
            with_profiles(&[(claude_work(), ProfileStatus::NotLoggedIn)]);
        state.set_pointer(place());
        let open = |state: &mut AgentsState| {
            reduce(state, AgentsIntent::OpenProfileMenu(claude_work()))
        };

        let opened = open(&mut state);
        let menu = open_menu(&state);
        let _ = reduce(&mut state, AgentsIntent::EditProfile(claude_work()));
        let after_edit = open_menu(&state);
        let is_editing = state.dialog().is_some();
        let _ = reduce(
            &mut state,
            AgentsIntent::Dialog(
                crate::profile_dialog::ProfileDialogIntent::Cancelled,
            ),
        );
        let _ = open(&mut state);
        let logged_in = reduce(&mut state, AgentsIntent::LogIn(claude_work()));
        let after_log_in = open_menu(&state);
        let _ = open(&mut state);
        let _ = reduce(&mut state, AgentsIntent::RemoveProfile(claude_work()));

        assert_eq!(opened, vec![AgentsOutcome::MenuOpened]);
        assert_eq!(menu, Some(AgentsMenu::Profile(claude_work())));
        assert_eq!(after_edit, None);
        assert!(is_editing);
        assert!(matches!(
            logged_in.as_slice(),
            [AgentsOutcome::OpenLoginTab { .. }]
        ));
        assert_eq!(after_log_in, None);
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
