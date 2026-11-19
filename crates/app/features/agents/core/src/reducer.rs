use std::time::SystemTime;

use otty_agents::Profile;
use otty_app_domain::mode::Mode;

use super::event::{AgentsIntent, AgentsOutcome};
use super::menus::AgentsMenu;
use super::state::AgentsState;
use super::types::{FolderAsker, LoadedFolder};
use super::usage::UsageList;
use super::{
    found_report, need, new_session_dialog, profile_actions, profile_dialog,
    program_version, reads, real_failures, session_actions, stats_dialog,
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
            let mut outcomes = reads::on_open(state, SystemTime::now());
            outcomes.extend(program_version::ask(state));
            outcomes
        },
        AgentsIntent::Leave => {
            state.set_active(false);
            Vec::new()
        },
        AgentsIntent::Refresh => reads::load_profiles(state),
        AgentsIntent::ProfilesLoaded(loaded) => {
            let check = real_failures::profiles(loaded.problem.as_deref());
            let mut outcomes = state.told_mut().tell(vec![check]);

            let to_read = state.profiles_mut().apply_profiles(loaded);
            // A read must follow when `agents.json` changed during this
            // one, which may have read it before: a profile just saved and
            // selected may be missing from it, so the selection waits.
            let is_reload_wanted = state.profiles_mut().take_reload_wanted();
            if !is_reload_wanted {
                state.forget_selection_that_left();
            }
            state.select_first_profile_if_none();
            found_report::open_on_first_start(state);

            outcomes
                .extend(to_read.into_iter().map(AgentsOutcome::LoadSessions));
            outcomes.extend(reads::after_profiles(state, SystemTime::now()));
            outcomes.extend(program_version::ask(state));
            if is_reload_wanted {
                outcomes.extend(reads::load_profiles(state));
            }
            outcomes
        },
        AgentsIntent::ProfilesReadStopped => {
            let check = real_failures::profiles_read_stopped();
            let mut outcomes = state.told_mut().tell(vec![check]);

            state.profiles_mut().end_stopped_read();
            if state.profiles_mut().take_reload_wanted() {
                outcomes.extend(reads::load_profiles(state));
            }
            outcomes
        },
        AgentsIntent::SessionsLoaded(loaded) => {
            if !is_still_shown(state, &loaded.profile, "sessions") {
                return Vec::new();
            }

            let check = real_failures::sessions(&loaded);
            state.profiles_mut().apply_sessions(loaded);
            state.forget_selection_that_left();
            state.select_first_profile_if_none();

            let mut outcomes = state.told_mut().tell(vec![check]);
            outcomes.extend(reads::go_on_at_start(state, SystemTime::now()));
            outcomes
        },
        AgentsIntent::LimitsLoaded(loaded) => {
            if !is_still_shown(state, &loaded.profile, "limits") {
                return Vec::new();
            }

            let checks = real_failures::limits(&loaded);
            state.profiles_mut().apply_limits(loaded);

            let mut outcomes = state.told_mut().tell(checks.into());
            outcomes.extend(reads::after_limits(state, SystemTime::now()));
            outcomes
        },
        AgentsIntent::SpendLoaded(loaded) => {
            if !is_still_shown(state, &loaded.profile, "tokens") {
                return Vec::new();
            }

            state.profiles_mut().apply_spend(loaded);
            reads::go_on_at_start(state, SystemTime::now())
        },
        AgentsIntent::BillLoaded(loaded) => {
            let profiles = state.profiles();
            if !profiles
                .is_shown_with_admin_key(&loaded.profile, &loaded.admin_key)
            {
                let name = loaded.profile.name();
                log::debug!(
                    "a bill read with an admin key no longer set dropped: {name}"
                );
                return Vec::new();
            }

            state.profiles_mut().apply_bill(loaded);
            reads::go_on_at_start(state, SystemTime::now())
        },
        AgentsIntent::PricesLoaded(loaded) => {
            let check = real_failures::prices(&loaded);
            state.profiles_mut().apply_prices(loaded);

            let mut outcomes = state.told_mut().tell(vec![check]);
            outcomes.extend(reads::go_on_at_start(state, SystemTime::now()));
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
        AgentsIntent::OpenSessionProfile(index) => {
            session_actions::open_profile(state, index);
            Vec::new()
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
        // The answer goes to the dialog that asked; the other one never
        // sees it.
        AgentsIntent::FolderLookedUp(loaded) => {
            apply_folder_lookup(state, loaded);
            Vec::new()
        },
        AgentsIntent::SelectProfile(profile) => {
            state.select_profile(&profile);
            Vec::new()
        },
        AgentsIntent::SelectAllAccounts => {
            state.select_all_accounts();
            Vec::new()
        },
        AgentsIntent::SelectAllProfiles => {
            state.select_all_profiles();
            Vec::new()
        },
        AgentsIntent::SelectAccount(account) => {
            state.select_account(account);
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
        AgentsIntent::VersionLoaded(loaded) => {
            state.profiles_mut().apply_program_version(loaded);
            Vec::new()
        },
        AgentsIntent::CloseFoundReport => found_report::close(state),
        AgentsIntent::FoundReportNeed(action) => {
            found_report::act(state, action)
        },
        AgentsIntent::FixNeed(action) => need::act(state, action),
        AgentsIntent::ToggleAccountsSection => {
            state.toggle_accounts_section();
            Vec::new()
        },
        AgentsIntent::ToggleAgentsSection => {
            state.toggle_agents_section();
            Vec::new()
        },
        AgentsIntent::ToggleProjectsSection => {
            state.toggle_projects_section();
            Vec::new()
        },
        AgentsIntent::OpenPeriodMenu => {
            open_usage_list(state, UsageList::Period, AgentsMenu::UsagePeriod)
        },
        AgentsIntent::OpenMeasureMenu => {
            open_usage_list(state, UsageList::Measure, AgentsMenu::UsageMeasure)
        },
        AgentsIntent::PickPeriod(period) => {
            let _ = state.close_menu();
            state.shown_usage_mut().pick_period(period);
            Vec::new()
        },
        AgentsIntent::PickMeasure(measure) => {
            let _ = state.close_menu();
            state.shown_usage_mut().pick_measure(measure);
            Vec::new()
        },
        AgentsIntent::ShowUsageTab(tab) => {
            state.shown_usage_mut().show_tab(tab);
            Vec::new()
        },
        AgentsIntent::PickUsageRow(name) => {
            state.shown_usage_mut().pick_row(name);
            Vec::new()
        },
        AgentsIntent::HoverUsageRow(name) => {
            state.shown_usage_mut().hover_row(name);
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

/// Open the drop-down `list` of the usage block on screen: in a
/// statistics popup its list under it, since no menu shows over a dialog,
/// else its `menu` at the pointer.
fn open_usage_list(
    state: &mut AgentsState,
    list: UsageList,
    menu: AgentsMenu,
) -> Vec<AgentsOutcome> {
    if let Some(dialog) = state.stats_mut() {
        dialog.usage_mut().toggle_list(list);
        return Vec::new();
    }

    state.open_menu(menu);
    vec![AgentsOutcome::MenuOpened]
}

/// Take the answer about a typed folder into the field of the dialog that
/// asked, while it is open and the text is still the one asked about. In
/// the profile dialog a folder that is not there is no error: a new
/// profile may name a new folder.
fn apply_folder_lookup(state: &mut AgentsState, loaded: LoadedFolder) {
    let home = state.env().home().to_path_buf();
    let field = match loaded.query.asker {
        FolderAsker::NewSessionDialog => {
            state.new_session_mut().map(|dialog| dialog.folder_mut())
        },
        FolderAsker::ProfileDialog => {
            state.dialog_mut().map(|dialog| dialog.typed_folder_mut())
        },
    };
    let Some(field) = field else {
        return;
    };

    if !field.apply(loaded, &home) {
        log::debug!("the answer about a folder typed before was dropped");
    }
}

/// Whether `profile` is still shown. The answer of a read of `what` of a
/// profile no longer shown is dropped, and the log says so.
fn is_still_shown(state: &AgentsState, profile: &Profile, what: &str) -> bool {
    if state.profiles().is_shown(profile) {
        return true;
    }

    let name = profile.name();
    log::debug!("agent {what} of a profile no longer shown dropped: {name}");
    false
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use iced::{Point, Size};
    use jiff::tz::TimeZone;
    use otty_agents::{
        AdminKeyRef, AgentCommand, ProfileStatus, SavedProfiles,
    };
    use otty_ui_kit::components::context_menu::MenuPlace;

    use super::*;
    use crate::project_filter::ProjectFilterChange;
    use crate::testing::{
        agents, api_key, at, bill_loaded, bill_of, claude, claude_work, codex,
        codex_work, connected, env, five_hour_limits, folders, limits_answer,
        limits_loaded, load_bill, no_files, now, open_menu, open_tab,
        organization, profiles_loaded, profiles_with_accounts_loaded, read,
        select_project, session, sessions_loaded, spend_loaded, spend_of,
        two_agents_with_sessions, with_accounts, with_billed_claude,
        with_profiles,
    };
    use crate::types::{LoadedProfile, LoadedProfiles, LoadedSessions};

    // Reading the profiles and the sessions.

    #[test]
    fn entering_the_mode_reads_the_profiles_once_at_a_time() {
        let mut state = AgentsState::new(agents(), env());

        let entered =
            reduce(&mut state, AgentsIntent::ModeChanged(Mode::Agents));
        let again = reduce(&mut state, AgentsIntent::Refresh);

        assert!(state.is_active());
        assert!(state.profiles().is_loading());
        assert!(!state.profiles().is_loaded());
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
        assert!(state.profiles().is_loaded());
        assert!(!state.profiles().is_loading());
        let shown: Vec<_> = state
            .profiles()
            .shown()
            .iter()
            .map(|shown| shown.profile().name())
            .collect();
        assert_eq!(shown, ["Claude Code", "Codex", "Claude Code · Work"]);
        assert!(state.profiles().shown()[0].is_reading_sessions());
        assert!(!state.profiles().shown()[2].is_reading_sessions());
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
                time_zone: None,
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
        select_project(&mut state, "/work/otty");

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
        select_project(&mut state, "/work/site");

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
        let _ = state.profiles_mut().start_reload();

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
        let otty = &state.profiles().projects()[1];
        let ids: Vec<_> = otty
            .sessions()
            .iter()
            .map(|entry| entry.session().id().as_str())
            .collect();
        assert_eq!(ids, ["x1", "c1"]);
        assert_eq!(otty.sessions()[0].profile(), &codex());
        assert!(!state.profiles().shown()[0].is_reading_sessions());
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
                sessions: crate::types::SessionsRead::Failed(vec![no_files()]),
                missing_folders: Vec::new(),
            }),
        );

        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert_eq!(state.profiles().shown()[0].failures(), None);
        assert_eq!(
            state.profiles().shown()[1].failures(),
            Some(&[no_files()][..])
        );
        assert!(!state.profiles().shown()[1].is_reading_sessions());
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
                sessions: crate::types::SessionsRead::Failed(vec![no_files()]),
                missing_folders: Vec::new(),
            }),
        );

        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert!(state.profiles().shown()[0].failures().is_some());
    }

    #[test]
    fn an_agent_that_can_not_list_sessions_has_none_and_no_failure() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(LoadedSessions {
                profile: claude(),
                sessions: crate::types::SessionsRead::NoWay,
                missing_folders: Vec::new(),
            }),
        );

        assert!(state.profiles().projects().is_empty());
        assert_eq!(state.profiles().shown()[0].failures(), None);
        assert!(!state.profiles().shown()[0].is_reading_sessions());
    }

    #[test]
    fn sessions_of_a_profile_no_longer_shown_are_dropped() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(
            &mut state,
            sessions_loaded(&codex(), vec![session("x1", "/work/otty", 1)]),
        );

        assert!(state.profiles().projects().is_empty());
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

        let crate::types::LimitsRead::Answered(expected) = answer else {
            panic!("the source answers");
        };
        assert_eq!(state.profiles().shown()[0].limits().last_good(), None);
        assert_eq!(
            state.profiles().shown()[1].limits().last_good(),
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
            spend_loaded(
                &codex(),
                crate::types::SpendCount::Counted(spend_of(10, 300)),
                at(10),
            ),
        );
        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let refreshed = reduce(
            &mut state,
            profiles_loaded(&[(claude(), connected()), (codex(), connected())]),
        );

        assert_eq!(state.profiles().shown()[0].spend().last_good(), None);
        assert_eq!(
            state.profiles().shown()[1].spend().last_good(),
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

    /// A read of the profiles that stopped before it answered keeps the
    /// profiles shown, tells why, and lets the next read start.
    #[test]
    fn a_profiles_read_that_stopped_keeps_the_profiles_and_tells_why() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let started = reduce(&mut state, AgentsIntent::Refresh);

        let stopped = reduce(&mut state, AgentsIntent::ProfilesReadStopped);
        let again = reduce(&mut state, AgentsIntent::Refresh);

        let told = otty_app_i18n::with_error(
            otty_app_i18n::Key::TplAgentsProfilesReadFailed,
            otty_app_i18n::t(otty_app_i18n::Key::AgentsReadStopped),
        );
        assert!(started.contains(&AgentsOutcome::LoadProfiles));
        assert_eq!(stopped, vec![AgentsOutcome::Failed(told)]);
        assert_eq!(state.profiles().shown().len(), 1);
        assert_eq!(state.profiles().shown()[0].profile(), &claude());
        assert!(again.contains(&AgentsOutcome::LoadProfiles));
    }

    /// The first read at the start of OTTY stopped before any answer: the
    /// profiles stay unread, so the mode shows that they load and not that
    /// there is no agent, and opening the mode reads them again.
    #[test]
    fn a_first_profiles_read_that_stopped_leaves_them_unread() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::LoadInBackground);

        let _ = reduce(&mut state, AgentsIntent::ProfilesReadStopped);
        let is_loaded = state.profiles().is_loaded();
        let entered = reduce(&mut state, AgentsIntent::Enter);

        assert!(!is_loaded);
        assert!(!state.profiles().are_fresh(SystemTime::now()));
        assert_eq!(entered, vec![AgentsOutcome::LoadProfiles]);
    }

    /// Only the name of a profile changed: it is the same profile, so its
    /// projects and tokens stay on screen through the reload.
    #[test]
    fn a_renamed_profile_keeps_what_was_read_of_it() {
        let renamed = Profile::default_for(
            claude().agent(),
            "Claude Code · Home",
            claude().config_dir().to_path_buf(),
        );
        let mut state = with_profiles(&[(claude(), api_key())]);
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                crate::types::SpendCount::Counted(spend_of(1, 2)),
                at(10),
            ),
        );

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let _ = reduce(
            &mut state,
            profiles_loaded(&[(renamed.clone(), api_key())]),
        );

        let shown = &state.profiles().shown()[0];
        assert_eq!(shown.profile(), &renamed);
        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert_eq!(shown.spend().last_good(), Some((&spend_of(1, 2), at(10))));
    }

    /// A read asked for before the profile was renamed still lands on it,
    /// and no second read of the same kind is asked for.
    #[test]
    fn a_read_on_its_way_lands_after_the_profile_was_renamed() {
        let renamed = Profile::default_for(
            claude().agent(),
            "Claude Code · Home",
            claude().config_dir().to_path_buf(),
        );
        let mut state = with_profiles(&[(claude(), connected())]);

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let reloaded = reduce(
            &mut state,
            profiles_loaded(&[(renamed.clone(), connected())]),
        );
        read(
            &mut state,
            &claude(),
            vec![session("c1", "/work/otty", 100)],
        );

        assert!(
            !reloaded.contains(&AgentsOutcome::LoadSessions(renamed)),
            "a read is on its way"
        );
        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert!(!state.profiles().shown()[0].is_reading_sessions());
    }

    /// A new admin key reads another organization's bill: the bill of the
    /// old key is forgotten, even when its answer comes late.
    #[test]
    fn a_new_admin_key_forgets_the_bill_of_the_old_one() {
        let other_key = AdminKeyRef::EnvVar(String::from("OTHER_ADMIN_KEY"));
        let mut state = with_billed_claude();
        let _ = reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(12, 3))));
        let _ = reduce(&mut state, AgentsIntent::LimitsTick);

        let _ = reduce(&mut state, AgentsIntent::Refresh);
        let reloaded = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(claude(), api_key(), vec![organization()])],
                SavedProfiles::default().with_admin_key_of(
                    &organization(),
                    Some(other_key.clone()),
                ),
            ),
        );
        let late =
            reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(1, 2))));

        assert!(reloaded.contains(&AgentsOutcome::LoadBill {
            profile: claude(),
            admin_key: other_key,
        }));
        assert!(late.is_empty(), "the bill of the old key is dropped");
        assert_eq!(state.profiles().shown()[0].bill().answer(), None);
    }

    #[test]
    fn the_time_zone_read_with_the_profiles_is_kept_until_known_again() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);
        let moscow = crate::testing::moscow();
        let read = |time_zone: Option<TimeZone>| {
            AgentsIntent::ProfilesLoaded(LoadedProfiles {
                profiles: Vec::new(),
                saved: None,
                problem: None,
                time_zone,
                read_at: at(0),
            })
        };
        let before = state.profiles().time_zone().cloned();

        let _ = reduce(&mut state, read(Some(moscow.clone())));
        let _ = reduce(&mut state, read(None));

        assert_eq!(before, None, "not known before the first read");
        assert_eq!(state.profiles().time_zone(), Some(&moscow));
    }

    #[test]
    fn the_tick_of_the_limits_does_not_count_the_tokens() {
        let mut state = with_profiles(&[(claude(), connected())]);
        let _ = reduce(
            &mut state,
            spend_loaded(
                &claude(),
                crate::types::SpendCount::Counted(spend_of(1, 2)),
                at(10),
            ),
        );

        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(!ticked.contains(&AgentsOutcome::LoadSpend(claude())));
    }

    // Reading the bill.

    #[test]
    fn an_api_key_profile_whose_organization_has_an_admin_key_has_its_bill_read()
     {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[
                    (claude(), api_key(), vec![organization()]),
                    (claude_work(), api_key(), Vec::new()),
                    (codex(), api_key(), vec![organization()]),
                ],
                crate::testing::saved_with_admin_key(),
            ),
        );

        let bills: Vec<_> = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AgentsOutcome::LoadBill { .. }))
            .collect();
        assert_eq!(
            bills,
            [&load_bill(&claude())],
            "a profile with no organization, or of an agent with no bill, \
             has no bill read"
        );
    }

    #[test]
    fn a_subscription_with_an_admin_key_has_no_bill_read() {
        let mut state = AgentsState::new(agents(), env());
        let _ = reduce(&mut state, AgentsIntent::Enter);

        let outcomes = reduce(
            &mut state,
            profiles_with_accounts_loaded(
                &[(claude(), connected(), vec![organization()])],
                crate::testing::saved_with_admin_key(),
            ),
        );

        assert!(!outcomes.contains(&load_bill(&claude())));
    }

    #[test]
    fn the_bill_is_kept_by_its_profile_and_read_again_on_the_tick() {
        let mut state = with_billed_claude();
        let while_reading = reduce(&mut state, AgentsIntent::LimitsTick);

        let applied =
            reduce(&mut state, bill_loaded(&claude(), Ok(bill_of(12, 3))));
        let ticked = reduce(&mut state, AgentsIntent::LimitsTick);

        assert!(while_reading.is_empty(), "a read is on its way");
        assert!(applied.is_empty());
        assert_eq!(
            state.profiles().shown()[0].bill().answer(),
            Some(&bill_of(12, 3))
        );
        assert_eq!(ticked, vec![load_bill(&claude())]);
    }

    #[test]
    fn a_bill_of_a_profile_no_longer_shown_is_dropped() {
        let mut state = with_billed_claude();

        let outcomes =
            reduce(&mut state, bill_loaded(&codex(), Ok(bill_of(1, 2))));

        assert!(outcomes.is_empty());
        assert_eq!(state.profiles().shown()[0].bill().answer(), None);
    }

    #[test]
    fn tokens_of_a_profile_no_longer_shown_are_dropped() {
        let mut state = with_profiles(&[(claude(), connected())]);

        let outcomes = reduce(
            &mut state,
            spend_loaded(
                &codex(),
                crate::types::SpendCount::Counted(spend_of(1, 2)),
                at(10),
            ),
        );

        assert!(outcomes.is_empty());
        assert_eq!(state.profiles().shown()[0].spend().last_good(), None);
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
        assert_eq!(state.profiles().shown()[0].limits().last_good(), None);
        assert!(state.profiles().shown()[0].limits().reads().is_reading());
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
                    accounts: Vec::new(),
                    version: Some(String::from(
                        crate::testing::SESSION_VERSION,
                    )),
                }],
                saved: Some(otty_agents::SavedProfiles::default()),
                problem: None,
                time_zone: None,
                read_at: at(0),
            }),
        );

        assert_eq!(outcomes, vec![AgentsOutcome::LoadSessions(codex())]);
        assert_eq!(folders(&state), [Path::new("/work/otty")]);
        assert!(state.profiles().shown()[0].is_reading_sessions());
    }

    // The sections, the search field and the menus of the left panel.

    fn place() -> MenuPlace {
        MenuPlace::new(Point::new(120.0, 300.0), Size::new(1200.0, 800.0))
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
            [AgentsOutcome::OpenTab { .. }]
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
    fn the_accounts_section_opens_and_closes_on_its_own() {
        let mut state = two_agents_with_sessions();
        assert!(state.is_accounts_open());

        let _ = reduce(&mut state, AgentsIntent::ToggleAccountsSection);
        assert!(!state.is_accounts_open());
        assert!(state.is_agents_open());
        assert!(state.is_projects_open());

        let _ = reduce(&mut state, AgentsIntent::ToggleAccountsSection);
        assert!(state.is_accounts_open());
    }

    /// One thing is selected at a time: All accounts, All profiles, an
    /// account, a profile or a project.
    #[test]
    fn each_selection_takes_the_place_of_the_one_before() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            SavedProfiles::default(),
        );

        let _ = reduce(&mut state, AgentsIntent::SelectAllAccounts);
        let all_accounts = (
            state.is_all_accounts_selected(),
            state.selected_profile().is_some(),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectAllProfiles);
        let all_profiles = (
            state.is_all_accounts_selected(),
            state.is_all_profiles_selected(),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectAccount(organization()));

        assert_eq!(all_accounts, (true, false));
        assert_eq!(all_profiles, (false, true));
        assert!(!state.is_all_profiles_selected());
        assert_eq!(
            state.selected_account().map(|account| account.provider()),
            Some(otty_agents::Provider::Anthropic)
        );
    }

    /// An account that is no longer found leaves the selection empty, so
    /// the first profile takes its place.
    #[test]
    fn an_account_no_longer_found_gives_its_place_to_the_first_profile() {
        let mut state = with_accounts(
            &[(claude(), connected(), vec![organization()])],
            SavedProfiles::default(),
        );
        let _ = reduce(&mut state, AgentsIntent::SelectAccount(organization()));

        let _ = reduce(&mut state, profiles_loaded(&[(claude(), connected())]));

        assert_eq!(state.selected_account(), None);
        assert_eq!(
            state.selected_profile().map(|shown| shown.profile()),
            Some(&claude())
        );
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
            .shown(state.profiles().projects(), |_| false, now())
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
        let shown =
            state
                .search()
                .shown(state.profiles().projects(), |_| false, now());
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

        select_project(&mut state, "/work/site");
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

        select_project(&mut state, "/work/otty");
        let _ = reduce(&mut state, AgentsIntent::SelectProfile(claude()));
        select_project(&mut state, "/work/otty");

        assert_eq!(
            state.selected_project().map(|project| project.folder()),
            Some(Path::new("/work/otty"))
        );
        assert!(state.search().is_open());
        assert_eq!(state.search().query(), "otty");
    }
}
