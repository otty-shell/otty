//! The sessions, limits and statistics of a profile as an agent part
//! reads them. Each is one read of the part; what the part answers it can
//! not do becomes the error of that read.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use otty_plugin_host::CallBatch;
use otty_plugin_host::agent::{Login, LoginKind, ReadError};

use crate::agent::AgentId;
use crate::called_part::CalledPart;
use crate::error::{SourceError, Unavailable};
use crate::limits::{LimitSource, Limits};
use crate::local_time::LocalNow;
use crate::plugin_failure::PluginFailureKind;
use crate::plugin_values::{limits_of, models_of, part_profile, time_of};
use crate::profile::Profile;
use crate::session::{AgentSession, SessionId, SessionSource};
use crate::source_id::SourceId;
use crate::stats::{ProjectStats, SessionStats, StatsSource};

/// The sessions of a profile, as the part lists them.
pub(crate) struct PluginSessions {
    part: Arc<CalledPart>,
}

impl PluginSessions {
    /// The sessions `part` lists.
    pub(crate) fn new(part: Arc<CalledPart>) -> Self {
        Self { part }
    }
}

impl SessionSource for PluginSessions {
    fn id(&self) -> SourceId {
        SourceId::AgentPart
    }

    fn sessions(
        &self,
        profile: &Profile,
    ) -> Result<Vec<AgentSession>, SourceError> {
        let asked = part_profile(profile);

        let sessions = self
            .part
            .read("sessions", |batch| batch.agent()?.sessions(&asked))?
            .map_err(|error| {
                read_error(
                    &self.part,
                    "sessions",
                    &error,
                    SourceError::NotAvailable(Unavailable::NoSessionFiles),
                )
            })?;

        let sessions = sessions
            .into_iter()
            .map(|session| {
                AgentSession::new(
                    SessionId::new(session.id),
                    PathBuf::from(session.folder),
                    session.title,
                    time_of(session.changed_ms),
                )
            })
            .collect();
        Ok(sessions)
    }
}

/// The limits the agent itself shows for the profile's login, from
/// wherever the part reads them.
pub(crate) struct PluginLimits {
    part: Arc<CalledPart>,
    starts_programs: bool,
}

impl PluginLimits {
    /// The limits `part` reads; the read starts the programs and hidden
    /// terminals the part declares only when `starts_programs`.
    pub(crate) fn new(part: Arc<CalledPart>, starts_programs: bool) -> Self {
        Self {
            part,
            starts_programs,
        }
    }
}

impl LimitSource for PluginLimits {
    /// The limits of the profile's subscription login, else of its first
    /// login, in one read of the part; `None` when the part reads no
    /// limits.
    fn limits(&self, profile: &Profile) -> Result<Option<Limits>, SourceError> {
        let asked = part_profile(profile);

        let read = |batch: &mut CallBatch<'_>| {
            let mut calls = batch.agent()?;
            let logins = calls.logins(&asked)?;
            let Some(login) = limits_login(&logins) else {
                return Ok(Err(SourceError::NotLoggedIn));
            };

            let limits = match calls.read_limits(&asked, &login.place)? {
                Ok(limits) => Ok(Some(limits_of(&limits))),
                Err(ReadError::NotSupported) => Ok(None),
                Err(error) => Err(read_error(
                    &self.part,
                    "read-limits",
                    &error,
                    no_limits(),
                )),
            };
            Ok(limits)
        };

        if self.starts_programs {
            self.part.read("read-limits", read)?
        } else {
            self.part.read_starting_nothing("read-limits", read)?
        }
    }
}

/// The statistics of a session and of a project folder, as the part
/// reads them.
pub(crate) struct PluginStats {
    part: Arc<CalledPart>,
}

impl PluginStats {
    /// The statistics `part` reads.
    pub(crate) fn new(part: Arc<CalledPart>) -> Self {
        Self { part }
    }

    /// What a part that reads no statistics gives.
    fn no_statistics(&self) -> SourceError {
        let agent = AgentId::new(self.part.part().id());

        SourceError::NotAvailable(Unavailable::NoStatistics(agent))
    }
}

impl StatsSource for PluginStats {
    fn session_stats(
        &self,
        profile: &Profile,
        session: &SessionId,
        _now: &LocalNow,
    ) -> Result<SessionStats, SourceError> {
        let asked = part_profile(profile);
        let call = "read-session-stats";

        let stats = self
            .part
            .read(call, |batch| {
                batch.agent()?.read_session_stats(&asked, session.as_str())
            })?
            .map_err(|error| {
                read_error(&self.part, call, &error, self.no_statistics())
            })?;

        Ok(SessionStats::new(
            stats.started_ms.map(time_of),
            stats.last_active_ms.map(time_of),
            stats.messages,
            models_of(&stats.models),
        ))
    }

    fn project_stats(
        &self,
        profile: &Profile,
        folder: &Path,
    ) -> Result<ProjectStats, SourceError> {
        let asked = part_profile(profile);
        let folder = folder.display().to_string();
        let call = "read-project-stats";

        let stats = self
            .part
            .read(call, |batch| {
                batch.agent()?.read_project_stats(&asked, &folder)
            })?
            .map_err(|error| {
                read_error(&self.part, call, &error, self.no_statistics())
            })?;

        Ok(ProjectStats::new(
            stats.first_ms.map(time_of),
            stats.last_ms.map(time_of),
            models_of(&stats.models),
        ))
    }
}

/// The error of `call` of `part` that answered `error`: `not_supported`
/// when the part does not read this, a refused key as such, anything else
/// as the plugin could not read.
pub(crate) fn read_error(
    part: &CalledPart,
    call: &'static str,
    error: &ReadError,
    not_supported: SourceError,
) -> SourceError {
    match error {
        ReadError::NotSupported => not_supported,
        ReadError::KeyRefused => SourceError::KeyRefused,
        _ => part.failure(call, PluginFailureKind::of_read(error)).into(),
    }
}

/// The login whose limits the part reads: a subscription, else the first.
fn limits_login(logins: &[Login]) -> Option<&Login> {
    logins
        .iter()
        .find(|login| matches!(login.kind, LoginKind::Subscription))
        .or_else(|| logins.first())
}

/// The error of "not supported" that [`read_error`] wants for the limits.
/// It is never given: [`PluginLimits`] answers "not supported" as `None`,
/// no limits, before it asks [`read_error`].
fn no_limits() -> SourceError {
    SourceError::NotAvailable(Unavailable::NoAccountSource)
}
