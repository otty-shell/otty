//! The statistics dialog: what it is about, a session or the selected
//! project, and what was read for it; opening it asks for the read, and
//! its answer fills it (spec Decisions 14 and 15).

use std::path::PathBuf;

use otty_agents::{
    Profile, Project, ProjectSession, ProjectStats, SessionId, SessionStats,
    SourceError,
};

use super::event::AgentsOutcome;
use super::profile_state::ProfileState;
use super::state::AgentsState;
use super::types::{
    LoadedProjectStats, LoadedSessionStats, ProjectStatsFailure,
};

/// The open statistics dialog.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StatsDialog {
    subject: StatsSubject,
    read: StatsRead,
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

    /// The dialog about `subject`, while its statistics are read.
    fn reading(subject: StatsSubject) -> Self {
        Self {
            subject,
            read: StatsRead::Reading,
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
        Ok(stats) => StatsRead::Session(stats),
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
            1,
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
