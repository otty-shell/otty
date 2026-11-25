//! Statistics of one session and of one project folder, which OTTY sums
//! from the answers in the agent's own files (spec Decisions 14 and 15;
//! issue 22: a plugin gives the answers, OTTY sums them).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::agent::{Agent, agent_of};
use crate::api_prices::ApiPrices;
use crate::error::{SourceError, Unavailable};
use crate::limits::WindowId;
use crate::local_time::LocalNow;
use crate::percent::Percent;
use crate::profile::Profile;
use crate::session::SessionId;
use crate::spend::{ModelTokens, Spend, merge_models, total_tokens};
use crate::token_count::TokenCount;

/// What the answers of one session tell: when its first and its last
/// answer were written, the tokens of each model, its tokens by day and
/// today by the hour, and, for an agent that writes them, how its usage
/// windows moved while it ran.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionStats {
    started_at: Option<SystemTime>,
    last_active_at: Option<SystemTime>,
    models: Vec<ModelTokens>,
    spend: Spend,
    limit_changes: Vec<LimitChange>,
}

impl SessionStats {
    /// When the first answer was written; `None` when the session has no
    /// answer.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    /// When the last answer was written.
    pub fn last_active_at(&self) -> Option<SystemTime> {
        self.last_active_at
    }

    /// How long the session ran, from its start to its last activity.
    pub fn length(&self) -> Option<Duration> {
        let started_at = self.started_at?;
        let last_active_at = self.last_active_at?;

        last_active_at.duration_since(started_at).ok()
    }

    /// How many answers the model gave: the requests of every model.
    pub fn answers(&self) -> u64 {
        self.models.iter().map(ModelTokens::requests).sum()
    }

    /// The tokens of each model, the most first.
    pub fn models(&self) -> &[ModelTokens] {
        &self.models
    }

    /// The tokens of every model together.
    pub fn tokens(&self) -> TokenCount {
        total_tokens(&self.models)
    }

    /// The tokens of the session by day over the last 30 days and today by
    /// the hour, in the user's time zone, as a profile's [`Spend`] counts
    /// them. Empty days when the session is older.
    pub fn spend(&self) -> &Spend {
        &self.spend
    }

    /// How each usage window moved from the first record of the limits in
    /// the session to the last one, shorter windows first. Empty for an
    /// agent that writes no limits into its sessions.
    pub fn limit_changes(&self) -> &[LimitChange] {
        &self.limit_changes
    }

    /// The statistics of a session that ran from `started_at` to
    /// `last_active_at`, with the tokens of `models`, the most first.
    ///
    /// ```
    /// use std::time::{Duration, SystemTime};
    ///
    /// use otty_agents::{ModelTokens, SessionStats, TokenCount};
    ///
    /// let start = SystemTime::UNIX_EPOCH;
    /// let stats = SessionStats::new(
    ///     Some(start),
    ///     Some(start + Duration::from_secs(600)),
    ///     vec![ModelTokens::new(
    ///         Some("gpt-5"),
    ///         TokenCount::new(10, 0, 2),
    ///         4,
    ///     )],
    /// );
    ///
    /// assert_eq!(stats.length(), Some(Duration::from_secs(600)));
    /// assert_eq!(stats.answers(), 4);
    /// ```
    pub fn new(
        started_at: Option<SystemTime>,
        last_active_at: Option<SystemTime>,
        models: Vec<ModelTokens>,
    ) -> Self {
        Self {
            started_at,
            last_active_at,
            models,
            spend: Spend::default(),
            limit_changes: Vec::new(),
        }
    }

    /// The same statistics with the tokens of the session by day and
    /// today by the hour.
    pub fn with_spend(self, spend: Spend) -> Self {
        Self { spend, ..self }
    }

    /// The same statistics with how the usage windows moved.
    pub fn with_limit_changes(self, limit_changes: Vec<LimitChange>) -> Self {
        Self {
            limit_changes,
            ..self
        }
    }
}

/// How much of one usage window was used when a session started and when
/// it was last active, such as "5 hours: 12 % → 31 %".
#[derive(Debug, Clone, PartialEq)]
pub struct LimitChange {
    window: WindowId,
    at_start: Percent,
    at_end: Percent,
}

impl LimitChange {
    /// Which window it is, such as the five-hour or the weekly one.
    pub fn window(&self) -> &WindowId {
        &self.window
    }

    /// How much was used at the first record of the session, from 0 to
    /// 100.
    pub fn at_start(&self) -> Percent {
        self.at_start
    }

    /// How much was used at the last record of the session, from 0 to 100.
    pub fn at_end(&self) -> Percent {
        self.at_end
    }

    /// The `window`, `at_start` percent used at the start and `at_end` at
    /// the end.
    ///
    /// ```
    /// use otty_agents::{LimitChange, Percent, WindowId};
    ///
    /// let start = Percent::new(12.0);
    /// let change =
    ///     LimitChange::new(WindowId::hours(5), start, Percent::new(31.0));
    ///
    /// assert_eq!(change.at_end().rounded(), 31);
    /// ```
    pub fn new(window: WindowId, at_start: Percent, at_end: Percent) -> Self {
        Self {
            window,
            at_start,
            at_end,
        }
    }
}

/// What the answers given in a project folder tell over the whole history:
/// when the first and the last answer were written, and the tokens of each
/// model. One profile reads its own part; [`merged`](Self::merged) adds the
/// parts of several profiles up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectStats {
    first_at: Option<SystemTime>,
    last_at: Option<SystemTime>,
    models: Vec<ModelTokens>,
}

impl ProjectStats {
    /// When the first answer was written; `None` when there is none.
    pub fn first_at(&self) -> Option<SystemTime> {
        self.first_at
    }

    /// When the last answer was written.
    pub fn last_at(&self) -> Option<SystemTime> {
        self.last_at
    }

    /// The tokens of each model, the most first.
    pub fn models(&self) -> &[ModelTokens] {
        &self.models
    }

    /// The tokens of every model together.
    pub fn tokens(&self) -> TokenCount {
        total_tokens(&self.models)
    }

    /// The statistics of answers written from `first_at` to `last_at`
    /// with the tokens of `models`, the most first.
    pub fn new(
        first_at: Option<SystemTime>,
        last_at: Option<SystemTime>,
        models: Vec<ModelTokens>,
    ) -> Self {
        Self {
            first_at,
            last_at,
            models,
        }
    }

    /// The statistics of `parts` together: the earliest first answer, the
    /// latest last one, and the tokens of the same model added up.
    ///
    /// ```
    /// use std::time::{Duration, SystemTime};
    ///
    /// use otty_agents::{ModelTokens, ProjectStats, TokenCount};
    ///
    /// let at = |seconds| SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
    /// let gpt = |input| {
    ///     vec![ModelTokens::new(
    ///         Some("gpt-5"),
    ///         TokenCount::new(input, 0, 0),
    ///         1,
    ///     )]
    /// };
    /// let work = ProjectStats::new(Some(at(10)), Some(at(20)), gpt(5));
    /// let home = ProjectStats::new(Some(at(5)), Some(at(15)), gpt(7));
    ///
    /// let both = ProjectStats::merged([&work, &home]);
    ///
    /// assert_eq!(both.first_at(), Some(at(5)));
    /// assert_eq!(both.last_at(), Some(at(20)));
    /// assert_eq!(both.tokens().input(), 12);
    /// ```
    pub fn merged<'a>(
        parts: impl IntoIterator<Item = &'a ProjectStats>,
    ) -> Self {
        let parts: Vec<&ProjectStats> = parts.into_iter().collect();

        let first_at = parts.iter().filter_map(|part| part.first_at).min();
        let last_at = parts.iter().filter_map(|part| part.last_at).max();
        let models =
            merge_models(parts.iter().flat_map(|part| part.models.iter()));

        Self {
            first_at,
            last_at,
            models,
        }
    }
}

/// Makes the statistics of a session or a project folder from the answers
/// in the agent's own files, never changing them.
///
/// It is a trait because reading the answers crosses into files through
/// the agent part of a plugin. An agent that can not read statistics gives
/// none, and the mode turns off Show statistics for its sessions.
#[cfg_attr(test, mockall::automock)]
pub trait StatsSource: Send {
    /// The statistics of the session `session` of `profile`, its days
    /// counted up to `now` in the user's time zone, each answer priced by
    /// the agent part at `prices` when the agent did not write its cost.
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
        now: &LocalNow,
        prices: &Arc<ApiPrices>,
    ) -> Result<SessionStats, SourceError>;

    /// The statistics of every session of `profile` in the project
    /// `folder`, over the whole history, priced the same way.
    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
        prices: &Arc<ApiPrices>,
    ) -> Result<ProjectStats, SourceError>;
}

/// The statistics of the session `session` of `profile`, read by the
/// statistics source of its agent among `agents`, its days counted up to
/// `now` and its answers priced at `prices`. An agent OTTY does not know,
/// or one that reads no statistics, is "not available".
pub fn read_session_stats(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
    session: &SessionId,
    now: &LocalNow,
    prices: &Arc<ApiPrices>,
) -> Result<SessionStats, SourceError> {
    stats_source(agents, profile)?.session_stats(profile, session, now, prices)
}

/// The statistics of every session of `profile` in the project `folder`,
/// over the whole history, read the same way as
/// [`read_session_stats`].
pub fn read_project_stats(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
    folder: &Path,
    prices: &Arc<ApiPrices>,
) -> Result<ProjectStats, SourceError> {
    stats_source(agents, profile)?.project_stats(profile, folder, prices)
}

/// The statistics source of the agent of `profile` among `agents`; "not
/// available" for an agent OTTY does not know or one that reads no
/// statistics.
fn stats_source(
    agents: &[Box<dyn Agent>],
    profile: &Profile,
) -> Result<Box<dyn StatsSource>, SourceError> {
    agent_of(agents, profile.agent())
        .and_then(|agent| agent.stats_source())
        .ok_or_else(|| {
            SourceError::NotAvailable(Unavailable::NoStatistics(
                profile.agent().clone(),
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::agent::AgentId;
    use crate::testing::{
        MockAgent, at, mock_entry, no_prices, test_agents, utc_now,
    };

    fn model(name: &str, input: u64, requests: u64) -> ModelTokens {
        ModelTokens::new(Some(name), TokenCount::new(input, 0, 1), requests)
    }

    #[test]
    fn a_session_counts_its_answers_tokens_and_length() {
        let stats = SessionStats::new(
            Some(at(100)),
            Some(at(700)),
            vec![model("opus", 30, 3), model("haiku", 5, 1)],
        );

        assert_eq!(stats.length(), Some(Duration::from_secs(600)));
        assert_eq!(stats.answers(), 4);
        assert_eq!(stats.tokens(), TokenCount::new(35, 0, 2));
        assert!(stats.limit_changes().is_empty());
    }

    #[test]
    fn a_session_with_no_times_has_no_length() {
        let stats = SessionStats::new(None, None, Vec::new());

        assert_eq!(stats.length(), None);
        assert_eq!(stats.answers(), 0);
        assert!(stats.tokens().is_zero());
    }

    #[test]
    fn parts_of_a_project_add_up_by_model() {
        let work = ProjectStats::new(
            Some(at(50)),
            Some(at(90)),
            vec![model("opus", 30, 3), model("gpt-5", 4, 1)],
        );
        let home = ProjectStats::new(
            Some(at(10)),
            Some(at(60)),
            vec![model("opus", 10, 1)],
        );
        let empty = ProjectStats::default();

        let project = ProjectStats::merged([&work, &home, &empty]);

        assert_eq!(project.first_at(), Some(at(10)));
        assert_eq!(project.last_at(), Some(at(90)));
        assert_eq!(
            project.models(),
            [
                ModelTokens::new(Some("opus"), TokenCount::new(40, 0, 2), 4),
                model("gpt-5", 4, 1),
            ]
        );
        assert_eq!(project.tokens(), TokenCount::new(44, 0, 3));
    }

    /// A profile of an agent OTTY does not know.
    fn stranger() -> Profile {
        Profile::added(
            AgentId::new("stranger"),
            "Stranger",
            PathBuf::from("/home/user/.stranger"),
        )
    }

    #[test]
    fn an_agent_that_reads_no_statistics_is_not_available() {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("silent", "silent"));
        agent.expect_stats_source().returning(|| None);
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];
        let silent_id = AgentId::new("silent");
        let silent =
            Profile::added(silent_id.clone(), "Silent", PathBuf::from("/s"));

        let session = read_session_stats(
            &agents,
            &silent,
            &SessionId::new("a1"),
            &utc_now(),
            &no_prices(),
        );
        let project =
            read_project_stats(&agents, &silent, Path::new("/w"), &no_prices());

        let not_available =
            SourceError::NotAvailable(Unavailable::NoStatistics(silent_id));
        assert_eq!(session, Err(not_available.clone()));
        assert_eq!(project, Err(not_available));
    }

    #[test]
    fn an_unknown_agent_reads_no_statistics() {
        let session = read_session_stats(
            &test_agents(),
            &stranger(),
            &SessionId::new("a1"),
            &utc_now(),
            &no_prices(),
        );

        assert_eq!(
            session,
            Err(SourceError::NotAvailable(Unavailable::NoStatistics(
                stranger().agent().clone()
            )))
        );
    }

    #[test]
    fn the_statistics_come_from_the_source_of_the_profiles_agent() {
        let mut agent = MockAgent::new();
        agent
            .expect_entry()
            .return_const(mock_entry("stranger", "stranger"));
        agent.expect_stats_source().returning(|| {
            let mut source = MockStatsSource::new();
            source
                .expect_project_stats()
                .withf(|_, folder, _| folder == Path::new("/work/otty"))
                .return_once(|_, _, _| Ok(ProjectStats::default()));
            Some(Box::new(source))
        });
        let agents: Vec<Box<dyn Agent>> = vec![Box::new(agent)];

        let project = read_project_stats(
            &agents,
            &stranger(),
            Path::new("/work/otty"),
            &no_prices(),
        );

        assert_eq!(project, Ok(ProjectStats::default()));
    }
}
