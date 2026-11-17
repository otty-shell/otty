//! Statistics of one session and of one project folder, read from the
//! agent's own files (spec Decisions 14 and 15).

use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::error::SourceError;
use crate::profile::Profile;
use crate::session::SessionId;
use crate::spend::{ModelTokens, merge_models};
use crate::token_count::TokenCount;

/// What the files of one session tell: when it started and was last
/// active, how many messages the user wrote, the tokens of each model, and
/// for Codex how its usage windows moved while it ran.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionStats {
    started_at: Option<SystemTime>,
    last_active_at: Option<SystemTime>,
    messages: u64,
    models: Vec<ModelTokens>,
    limit_changes: Vec<LimitChange>,
}

impl SessionStats {
    /// When the first message or answer was written; `None` when the
    /// files tell no time.
    pub fn started_at(&self) -> Option<SystemTime> {
        self.started_at
    }

    /// When the last message or answer was written.
    pub fn last_active_at(&self) -> Option<SystemTime> {
        self.last_active_at
    }

    /// How long the session ran, from its start to its last activity.
    pub fn length(&self) -> Option<Duration> {
        let started_at = self.started_at?;
        let last_active_at = self.last_active_at?;

        last_active_at.duration_since(started_at).ok()
    }

    /// How many messages the user wrote.
    pub fn messages(&self) -> u64 {
        self.messages
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
        self.models
            .iter()
            .fold(TokenCount::default(), |sum, model| sum + model.tokens())
    }

    /// How each usage window moved from the first record of the limits in
    /// the session to the last one, shorter windows first. Empty for an
    /// agent that writes no limits into its sessions.
    pub fn limit_changes(&self) -> &[LimitChange] {
        &self.limit_changes
    }

    /// The statistics of a session that ran from `started_at` to
    /// `last_active_at`, with `messages` of the user and the tokens of
    /// `models`, the most first.
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
    ///     3,
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
        messages: u64,
        models: Vec<ModelTokens>,
    ) -> Self {
        Self {
            started_at,
            last_active_at,
            messages,
            models,
            limit_changes: Vec::new(),
        }
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
    name: String,
    at_start: f64,
    at_end: f64,
}

impl LimitChange {
    /// The name of the window, such as "5 hours" or "7 days".
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How much was used at the first record of the session, from 0 to
    /// 100.
    pub fn at_start(&self) -> f64 {
        self.at_start
    }

    /// How much was used at the last record of the session, from 0 to 100.
    pub fn at_end(&self) -> f64 {
        self.at_end
    }

    /// The window `name`, `at_start` percent used at the start and `at_end`
    /// at the end.
    ///
    /// ```
    /// use otty_agents::LimitChange;
    ///
    /// let change = LimitChange::new("5 hours", 12.0, 31.0);
    ///
    /// assert_eq!(change.at_end(), 31.0);
    /// ```
    pub fn new(name: impl Into<String>, at_start: f64, at_end: f64) -> Self {
        Self {
            name: name.into(),
            at_start,
            at_end,
        }
    }
}

/// What the session files of a project folder tell over the whole history:
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
        self.models
            .iter()
            .fold(TokenCount::default(), |sum, model| sum + model.tokens())
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

/// Reads the statistics of a session or a project folder from the agent's
/// own files, never changing them.
///
/// It is a trait because each agent writes its sessions its own way: Claude
/// Code and Codex each have one, and reading crosses into files. An agent
/// that can not read statistics gives none, and the mode turns off Show
/// statistics for its sessions.
#[cfg_attr(test, mockall::automock)]
pub trait StatsSource: Send {
    /// The statistics of the session `session` of `profile`.
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
    ) -> Result<SessionStats, SourceError>;

    /// The statistics of every session of `profile` in the project
    /// `folder`, over the whole history.
    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::at;

    fn model(name: &str, input: u64, requests: u64) -> ModelTokens {
        ModelTokens::new(Some(name), TokenCount::new(input, 0, 1), requests)
    }

    #[test]
    fn a_session_counts_its_answers_tokens_and_length() {
        let stats = SessionStats::new(
            Some(at(100)),
            Some(at(700)),
            2,
            vec![model("opus", 30, 3), model("haiku", 5, 1)],
        );

        assert_eq!(stats.length(), Some(Duration::from_secs(600)));
        assert_eq!(stats.messages(), 2);
        assert_eq!(stats.answers(), 4);
        assert_eq!(stats.tokens(), TokenCount::new(35, 0, 2));
        assert!(stats.limit_changes().is_empty());
    }

    #[test]
    fn a_session_with_no_times_has_no_length() {
        let stats = SessionStats::new(None, None, 0, Vec::new());

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
}
