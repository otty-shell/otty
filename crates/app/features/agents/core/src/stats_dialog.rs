//! The statistics dialog: what it is about, a session or the selected
//! project, what was read for it and the choices of its usage block;
//! opening it asks for the read, and its answer fills it (spec Decisions
//! 14 and 15, spec 2026-11-19 Decision 6).

use std::path::PathBuf;

use otty_agents::{
    Profile, Project, ProjectSession, ProjectStats, SessionId, SessionStats,
    SourceError, Spend,
};

use super::event::AgentsOutcome;
use super::profile_state::ProfileState;
use super::state::AgentsState;
use super::types::{
    LoadedProjectStats, LoadedSessionStats, ProjectStatsFailure,
};
use super::usage::{UsageChoice, UsagePeriod};

/// The open statistics dialog.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatsDialog {
    subject: StatsSubject,
    read: StatsRead,
    usage: UsageChoice,
}

impl StatsDialog {
    /// What the dialog is about.
    pub(crate) fn subject(&self) -> &StatsSubject {
        &self.subject
    }

    /// What was read for it so far.
    pub(crate) fn read(&self) -> &StatsRead {
        &self.read
    }

    /// What its usage block shows: its own choices, not those of the
    /// profile pages.
    pub(crate) fn usage(&self) -> &UsageChoice {
        &self.usage
    }

    /// The choices of its usage block, to change them.
    pub(crate) fn usage_mut(&mut self) -> &mut UsageChoice {
        &mut self.usage
    }

    /// The dialog about `subject`, while its statistics are read; its
    /// usage block starts on 7 days of tokens by model.
    fn reading(subject: StatsSubject) -> Self {
        Self {
            subject,
            read: StatsRead::Reading,
            usage: UsageChoice::default(),
        }
    }
}

/// What a statistics dialog is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StatsSubject {
    /// One session of a profile, with the title the table shows.
    Session {
        profile: Profile,
        session: SessionId,
        title: String,
    },
    /// The project in this folder.
    Project(PathBuf),
}

/// What was read for a statistics dialog.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StatsRead {
    /// The read is on its way: the dialog shows the spinner.
    Reading,
    /// The statistics of the session.
    Session(SessionStats),
    /// What each profile's files tell about the project, in the order of
    /// the profiles.
    Project(Vec<(Profile, ProjectStats)>),
    /// Why the statistics of the session could not be read.
    SessionFailed(SourceError),
    /// Why the statistics of the project could not be read, with the
    /// profile whose files failed.
    ProjectFailed(ProjectStatsFailure),
}

/// Open the statistics of the session of `entry` and ask for their read,
/// while its agent reads statistics.
pub(crate) fn show_session(
    state: &mut AgentsState,
    entry: &ProjectSession,
) -> Vec<AgentsOutcome> {
    let profile = entry.profile();
    if !state.has_stats(profile.agent()) {
        return Vec::new();
    }

    let session = entry.session().id().clone();
    state.open_stats(StatsDialog::reading(StatsSubject::Session {
        profile: profile.clone(),
        session: session.clone(),
        title: entry.session().title().to_string(),
    }));

    vec![AgentsOutcome::LoadSessionStats {
        profile: profile.clone(),
        session,
    }]
}

/// Open the statistics of the selected project and ask for their read,
/// even while its folder is gone: the history is still there.
pub(crate) fn show_project(state: &mut AgentsState) -> Vec<AgentsOutcome> {
    let Some(folder) = state.selected_project().map(|project| project.folder())
    else {
        return Vec::new();
    };
    let folder = folder.to_path_buf();
    let profiles = stats_profiles(state);
    if profiles.is_empty() {
        return Vec::new();
    }

    state.open_stats(StatsDialog::reading(StatsSubject::Project(
        folder.clone(),
    )));

    vec![AgentsOutcome::LoadProjectStats { folder, profiles }]
}

/// Whether the statistics of the selected project can be shown: a
/// profile makes them.
pub(crate) fn can_show_project_stats(state: &AgentsState) -> bool {
    let Some(project) = state.selected_project() else {
        return false;
    };

    state
        .profiles()
        .shown()
        .iter()
        .any(|shown| makes_project_stats(state, project, shown))
}

/// Take the statistics of a session, while the dialog is still about it.
pub(crate) fn apply_session(
    state: &mut AgentsState,
    loaded: LoadedSessionStats,
) {
    let Some(dialog) = state.stats_mut() else {
        return;
    };
    let is_about_it = matches!(
        &dialog.subject,
        StatsSubject::Session { profile, session, .. }
            if *profile == loaded.profile && *session == loaded.session
    );
    if !is_about_it {
        log::debug!("statistics of a session no longer shown dropped");
        return;
    }

    dialog.read = match loaded.stats {
        Ok(stats) => {
            if let Some(period) = period_of_last_tokens(stats.spend()) {
                dialog.usage.pick_period(period);
            }
            StatsRead::Session(stats)
        },
        Err(error) => StatsRead::SessionFailed(error),
    };
}

/// Take the statistics of a project, while the dialog is still about it.
pub(crate) fn apply_project(
    state: &mut AgentsState,
    loaded: LoadedProjectStats,
) {
    let Some(dialog) = state.stats_mut() else {
        return;
    };
    let is_about_it = matches!(
        &dialog.subject,
        StatsSubject::Project(folder) if *folder == loaded.folder
    );
    if !is_about_it {
        log::debug!("statistics of a project no longer shown dropped");
        return;
    }

    dialog.read = match loaded.parts {
        Ok(parts) => StatsRead::Project(parts),
        Err(failure) => StatsRead::ProjectFailed(failure),
    };
}

/// The shortest period that holds the last tokens of a session counted in
/// `spend`: Today, 7 days or 30 days; `None` when it has no tokens in the
/// last 30 days.
fn period_of_last_tokens(spend: &Spend) -> Option<UsagePeriod> {
    let days = spend.last_30_days_by_day();
    let last = days.iter().rposition(|day| !day.tokens().is_zero())?;

    let days_ago = days.len() - 1 - last;
    let period = match days_ago {
        0 => UsagePeriod::Today,
        1..7 => UsagePeriod::SevenDays,
        _ => UsagePeriod::ThirtyDays,
    };
    Some(period)
}

/// The profiles whose statistics make the selected project, each once,
/// in the order of the profiles.
fn stats_profiles(state: &AgentsState) -> Vec<Profile> {
    let Some(project) = state.selected_project() else {
        return Vec::new();
    };

    state
        .profiles()
        .shown()
        .iter()
        .filter(|shown| makes_project_stats(state, project, shown))
        .map(|shown| shown.profile().clone())
        .collect()
}

/// Whether the statistics of `shown` make those of `project`: it has a
/// session in it, its config folder is there and its agent reads
/// statistics.
fn makes_project_stats(
    state: &AgentsState,
    project: &Project,
    shown: &ProfileState,
) -> bool {
    shown.has_config_dir()
        && state.has_stats(shown.profile().agent())
        && project
            .sessions()
            .iter()
            .any(|entry| entry.profile() == shown.profile())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use otty_agents::{
        AgentId, ModelTokens, Profile, SourceError, TokenCount, Unavailable,
    };

    use super::*;
    use crate::event::{AgentsIntent, SessionAction};
    use crate::reducer::reduce;
    use crate::testing::{
        claude, codex, codex_work, connected, read, session,
        two_agents_with_sessions, with_profiles,
    };
    use crate::usage::{UsageList, UsageMeasure, UsagePeriod, UsageTab};

    fn otty() -> PathBuf {
        PathBuf::from("/work/otty")
    }

    /// The project `/work/otty` selected, with its session `index` of the
    /// table pressed with the right button.
    fn with_session_menu(index: usize) -> AgentsState {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::OpenSessionMenu(index));

        state
    }

    fn some_stats() -> SessionStats {
        SessionStats::new(
            None,
            None,
            vec![ModelTokens::new(Some("gpt-5"), TokenCount::new(1, 0, 1), 1)],
        )
    }

    #[test]
    fn show_statistics_of_a_session_opens_the_dialog_and_reads_them() {
        // The newest session of /work/otty is x1 of Codex.
        let mut state = with_session_menu(0);

        let outcomes = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LoadSessionStats {
                profile: codex(),
                session: SessionId::new("x1"),
            }]
        );
        assert!(state.menu().is_none());
        let dialog = state.stats().expect("the dialog opens");
        assert_eq!(dialog.read(), &StatsRead::Reading);
        assert_eq!(
            dialog.subject(),
            &StatsSubject::Session {
                profile: codex(),
                session: SessionId::new("x1"),
                title: String::from("Title of x1"),
            }
        );
    }

    #[test]
    fn a_session_of_an_agent_that_reads_no_statistics_opens_nothing() {
        let stranger = Profile::default_for(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        );
        let mut state = with_profiles(&[(stranger.clone(), connected())]);
        read(&mut state, &stranger, vec![session("s1", "/work/otty", 10)]);
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::OpenSessionMenu(0));

        let outcomes = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );

        assert_eq!(outcomes, Vec::new());
        assert!(state.stats().is_none());
    }

    #[test]
    fn the_answer_fills_the_dialog_and_a_failure_shows_its_reason() {
        let mut state = with_session_menu(0);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
        let loaded = |stats| {
            AgentsIntent::SessionStatsLoaded(LoadedSessionStats {
                profile: codex(),
                session: SessionId::new("x1"),
                stats,
            })
        };

        let _ = reduce(&mut state, loaded(Ok(some_stats())));
        let read_ok = state.stats().map(|dialog| dialog.read().clone());
        let no_file = SourceError::NotAvailable(Unavailable::NoSessionFile(
            SessionId::new("x1"),
        ));
        let _ = reduce(&mut state, loaded(Err(no_file)));
        let failed = state.stats().map(|dialog| dialog.read().clone());

        assert_eq!(read_ok, Some(StatsRead::Session(some_stats())));
        assert_eq!(
            failed,
            Some(StatsRead::SessionFailed(SourceError::NotAvailable(
                Unavailable::NoSessionFile(SessionId::new("x1"))
            )))
        );
    }

    #[test]
    fn an_answer_about_another_session_or_a_closed_dialog_is_dropped() {
        let mut state = with_session_menu(0);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
        let other = AgentsIntent::SessionStatsLoaded(LoadedSessionStats {
            profile: claude(),
            session: SessionId::new("c1"),
            stats: Ok(some_stats()),
        });

        let _ = reduce(&mut state, other.clone());
        let still_reading = state.stats().map(|dialog| dialog.read().clone());
        let _ = reduce(&mut state, AgentsIntent::CloseStats);
        let _ = reduce(&mut state, other);

        assert_eq!(still_reading, Some(StatsRead::Reading));
        assert!(state.stats().is_none());
    }

    /// The day `day` of November 2026 with 10 tokens of gpt-5.
    fn tokens_on(day: i8) -> otty_agents::DaySpend {
        otty_agents::DaySpend::new(
            jiff::civil::date(2026, 11, day),
            vec![ModelTokens::new(
                Some("gpt-5"),
                TokenCount::new(10, 0, 0),
                1,
            )],
        )
    }

    /// The days 2026-11-01 to 2026-11-16 of a session, with tokens only
    /// on `with_tokens`.
    fn session_spend(with_tokens: &[i8]) -> otty_agents::Spend {
        let days = (1..=16)
            .map(|day| {
                if with_tokens.contains(&day) {
                    tokens_on(day)
                } else {
                    otty_agents::DaySpend::new(
                        jiff::civil::date(2026, 11, day),
                        Vec::new(),
                    )
                }
            })
            .collect();

        otty_agents::Spend::new(days, Vec::new())
    }

    /// The period the session popup opens on once its statistics with
    /// tokens on `with_tokens` came.
    fn opening_period(with_tokens: &[i8]) -> UsagePeriod {
        let mut state = with_session_menu(0);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionMenu(SessionAction::ShowStatistics),
        );
        let stats = some_stats().with_spend(session_spend(with_tokens));

        let _ = reduce(
            &mut state,
            AgentsIntent::SessionStatsLoaded(LoadedSessionStats {
                profile: codex(),
                session: SessionId::new("x1"),
                stats: Ok(stats),
            }),
        );

        state.stats().expect("the popup is open").usage().period()
    }

    /// The prototype opens the session popup on the shortest period that
    /// holds the last tokens of the session.
    #[test]
    fn the_session_popup_opens_on_the_period_of_its_last_tokens() {
        assert_eq!(opening_period(&[3, 16]), UsagePeriod::Today);
        assert_eq!(opening_period(&[3, 12]), UsagePeriod::SevenDays);
        assert_eq!(opening_period(&[3, 9]), UsagePeriod::ThirtyDays);
        assert_eq!(opening_period(&[]), UsagePeriod::SevenDays);
    }

    #[test]
    fn the_popup_has_its_own_usage_choices_while_it_is_open() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);

        for intent in [
            AgentsIntent::ShowUsageTab(UsageTab::ByProfile),
            AgentsIntent::PickPeriod(UsagePeriod::Today),
            AgentsIntent::PickMeasure(UsageMeasure::Money),
            AgentsIntent::PickUsageRow(String::from("Codex")),
            AgentsIntent::HoverUsageRow(Some(String::from("Claude Code"))),
        ] {
            let _ = reduce(&mut state, intent);
        }
        let popup = state.stats().expect("the popup is open").usage().clone();
        let _ = reduce(&mut state, AgentsIntent::CloseStats);
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);

        assert_eq!(popup.tab(), UsageTab::ByProfile);
        assert_eq!(popup.period(), UsagePeriod::Today);
        assert_eq!(popup.measure(), UsageMeasure::Money);
        assert_eq!(popup.picked(), Some("Codex"));
        assert_eq!(popup.hovered(), Some("Claude Code"));
        assert_eq!(state.usage(), &crate::usage::UsageChoice::default());
        assert_eq!(
            state.stats().map(|dialog| dialog.usage().clone()),
            Some(crate::usage::UsageChoice::default()),
            "a popup opened again starts afresh"
        );
    }

    /// No menu of the mode shows over a dialog: the drop-downs of the
    /// popup open their lists under them.
    #[test]
    fn a_drop_down_of_the_popup_opens_its_list_and_no_menu() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);

        let opened = reduce(&mut state, AgentsIntent::OpenPeriodMenu);
        let open = state.stats().and_then(|dialog| dialog.usage().open_list());
        let _ = reduce(&mut state, AgentsIntent::OpenMeasureMenu);
        let other = state.stats().and_then(|dialog| dialog.usage().open_list());
        let _ =
            reduce(&mut state, AgentsIntent::PickMeasure(UsageMeasure::Money));
        let picked =
            state.stats().and_then(|dialog| dialog.usage().open_list());

        assert_eq!(opened, Vec::new());
        assert!(state.menu().is_none());
        assert_eq!(open, Some(UsageList::Period));
        assert_eq!(other, Some(UsageList::Measure));
        assert_eq!(picked, None);
    }

    #[test]
    fn project_statistics_read_every_profile_with_a_session_in_it() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));

        let outcomes = reduce(&mut state, AgentsIntent::ShowProjectStats);

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LoadProjectStats {
                folder: otty(),
                profiles: vec![claude(), codex()],
            }]
        );
        assert_eq!(
            state.stats().map(|dialog| dialog.subject().clone()),
            Some(StatsSubject::Project(otty()))
        );
    }

    #[test]
    fn project_statistics_stay_on_when_the_folder_is_gone() {
        let mut state = with_profiles(&[(codex_work(), connected())]);
        let _ = reduce(
            &mut state,
            AgentsIntent::SessionsLoaded(crate::types::LoadedSessions {
                profile: codex_work(),
                sessions: crate::types::SessionsRead::Read(vec![session(
                    "w1",
                    "/gone/app",
                    10,
                )]),
                missing_folders: vec![PathBuf::from("/gone/app")],
            }),
        );
        let _ = reduce(
            &mut state,
            AgentsIntent::SelectProject(PathBuf::from("/gone/app")),
        );

        let outcomes = reduce(&mut state, AgentsIntent::ShowProjectStats);

        assert_eq!(
            outcomes,
            vec![AgentsOutcome::LoadProjectStats {
                folder: PathBuf::from("/gone/app"),
                profiles: vec![codex_work()],
            }]
        );
    }

    #[test]
    fn the_project_answer_fills_the_dialog_and_close_closes_it() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);
        let parts = vec![(claude(), ProjectStats::default())];

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: PathBuf::from("/work/site"),
                parts: Ok(Vec::new()),
            }),
        );
        let other_dropped = state.stats().map(|dialog| dialog.read().clone());
        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Ok(parts.clone()),
            }),
        );
        let read = state.stats().map(|dialog| dialog.read().clone());
        let closed = reduce(&mut state, AgentsIntent::CloseStats);

        assert_eq!(other_dropped, Some(StatsRead::Reading));
        assert_eq!(read, Some(StatsRead::Project(parts)));
        assert_eq!(closed, Vec::new());
        assert!(state.stats().is_none());
    }

    #[test]
    fn a_failed_project_read_is_kept_with_the_profile_whose_files_failed() {
        let mut state = two_agents_with_sessions();
        let _ = reduce(&mut state, AgentsIntent::SelectProject(otty()));
        let _ = reduce(&mut state, AgentsIntent::ShowProjectStats);
        let failure = ProjectStatsFailure {
            profile_name: Some(claude().name().to_string()),
            error: SourceError::NotAvailable(Unavailable::ReadStopped),
        };

        let _ = reduce(
            &mut state,
            AgentsIntent::ProjectStatsLoaded(LoadedProjectStats {
                folder: otty(),
                parts: Err(failure.clone()),
            }),
        );

        assert_eq!(
            state.stats().map(|dialog| dialog.read().clone()),
            Some(StatsRead::ProjectFailed(failure))
        );
    }
}
